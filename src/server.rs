//! Código só do servidor: banco (Turso/libsql), sessões e avisos em tempo real.

use crate::model::{
    BoiView, Hand, Mode, Outcome, PairColors, Period, Player, RoomView, UserInfo, history_page,
    pair_key, resolve_side_colors, sides_of,
};
use crate::palette;
use dioxus::fullstack::{HeaderMap, HttpError, ServerFnError, StatusCode};
use libsql::{Connection, Database, params};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::{OnceCell, broadcast};

pub const SESSION_COOKIE: &str = "placar_session";

static DB: OnceCell<Database> = OnceCell::const_new();

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name  TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS sessions (
    token      TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS rooms (
    id         INTEGER PRIMARY KEY,
    code       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    mode       TEXT NOT NULL,
    owner_id   INTEGER NOT NULL REFERENCES users(id),
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
-- Jogadores da sala. user_id nulo = avulso. seat = posição atual na roda.
CREATE TABLE IF NOT EXISTS players (
    id      INTEGER PRIMARY KEY,
    room_id INTEGER NOT NULL REFERENCES rooms(id),
    name    TEXT NOT NULL,
    user_id INTEGER REFERENCES users(id),
    seat    INTEGER NOT NULL
);
-- seating: ids dos jogadores na ordem dos assentos, em JSON, congelada no início do boi.
CREATE TABLE IF NOT EXISTS bois (
    id         INTEGER PRIMARY KEY,
    room_id    INTEGER NOT NULL REFERENCES rooms(id),
    seating    TEXT NOT NULL,
    started_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE IF NOT EXISTS maos (
    id           INTEGER PRIMARY KEY,
    boi_id       INTEGER NOT NULL REFERENCES bois(id),
    seq          INTEGER NOT NULL,
    starter_seat INTEGER NOT NULL,
    outcome      TEXT NOT NULL,
    winner_side  INTEGER,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE (boi_id, seq)
);
-- Cores de cada dupla já formada. player_a < player_b.
CREATE TABLE IF NOT EXISTS pairs (
    room_id   INTEGER NOT NULL REFERENCES rooms(id),
    player_a  INTEGER NOT NULL REFERENCES players(id),
    player_b  INTEGER NOT NULL REFERENCES players(id),
    color     INTEGER NOT NULL,
    color2    INTEGER NOT NULL,
    chosen_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (room_id, player_a, player_b)
);
CREATE INDEX IF NOT EXISTS players_room ON players(room_id);
CREATE INDEX IF NOT EXISTS players_user ON players(user_id);
CREATE INDEX IF NOT EXISTS bois_room ON bois(room_id);
"#;

/// Erro de banco: registra no log e devolve uma mensagem genérica.
pub fn db_err(e: impl std::fmt::Display) -> ServerFnError {
    eprintln!("erro de banco: {e}");
    HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Erro no servidor. Tente de novo.",
    )
    .into()
}

pub fn user_err(status: StatusCode, msg: impl Into<String>) -> ServerFnError {
    HttpError::new(status, msg).into()
}

impl From<crate::room_rules::Refusal> for ServerFnError {
    fn from(r: crate::room_rules::Refusal) -> Self {
        use crate::room_rules::Refusal;
        match r {
            Refusal::Invalid(msg) => user_err(StatusCode::BAD_REQUEST, msg),
            Refusal::Conflict(msg) => user_err(StatusCode::CONFLICT, msg),
            Refusal::Forbidden(msg) => user_err(StatusCode::FORBIDDEN, msg),
        }
    }
}

/// Conexão com o banco. Usa o Turso se `TURSO_DATABASE_URL` estiver definida;
/// senão, um arquivo SQLite local (`placar.db`, ou o caminho em `PLACAR_DB`).
pub async fn conn() -> Result<Connection, ServerFnError> {
    let db = DB
        .get_or_try_init(|| async {
            let db = match std::env::var("TURSO_DATABASE_URL") {
                Ok(url) => {
                    let token = std::env::var("TURSO_AUTH_TOKEN").unwrap_or_default();
                    libsql::Builder::new_remote(url, token).build().await?
                }
                Err(_) => {
                    let path = std::env::var("PLACAR_DB").unwrap_or_else(|_| "placar.db".into());
                    libsql::Builder::new_local(path).build().await?
                }
            };
            let conn = db.connect()?;
            conn.execute_batch(SCHEMA).await?;
            migrate(&conn).await?;
            Ok::<_, libsql::Error>(db)
        })
        .await
        .map_err(db_err)?;
    db.connect().map_err(db_err)
}

/// Colunas acrescentadas depois da primeira versão do banco.
async fn migrate(conn: &Connection) -> Result<(), libsql::Error> {
    async fn has_column(
        conn: &Connection,
        table: &str,
        column: &str,
    ) -> Result<bool, libsql::Error> {
        let mut rows = conn
            .query(
                "SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2",
                params![table, column],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }
    if !has_column(conn, "players", "color").await? {
        // Salas antigas: cada jogador ganha a cor do seu lugar, todas diferentes.
        conn.execute_batch(
            "ALTER TABLE players ADD COLUMN color INTEGER NOT NULL DEFAULT 0;
             UPDATE players SET color = seat;",
        )
        .await?;
    }
    if !has_column(conn, "bois", "colors").await? {
        conn.execute(
            "ALTER TABLE bois ADD COLUMN colors TEXT NOT NULL DEFAULT '[]'",
            (),
        )
        .await?;
    }
    Ok(())
}

pub fn random_token(len: usize, alphabet: &[u8]) -> String {
    use rand::Rng;
    let mut rng = rand::rngs::OsRng;
    (0..len)
        .map(|_| alphabet[rng.gen_range(0..alphabet.len())] as char)
        .collect()
}

pub fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(db_err)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    PasswordHash::new(hash).is_ok_and(|h| {
        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}

pub async fn create_session(conn: &Connection, user_id: i64) -> Result<String, ServerFnError> {
    let token = random_token(
        40,
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
    );
    conn.execute(
        "INSERT INTO sessions (token, user_id) VALUES (?1, ?2)",
        params![token.clone(), user_id],
    )
    .await
    .map_err(db_err)?;
    Ok(token)
}

pub fn session_cookie(token: &str, max_age: u32) -> String {
    format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}")
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(dioxus::fullstack::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_string())
}

pub async fn current_user(
    conn: &Connection,
    headers: &HeaderMap,
) -> Result<Option<UserInfo>, ServerFnError> {
    let Some(token) = session_token(headers) else {
        return Ok(None);
    };
    let mut rows = conn
        .query(
            "SELECT u.id, u.username, u.display_name FROM sessions s
             JOIN users u ON u.id = s.user_id WHERE s.token = ?1",
            params![token],
        )
        .await
        .map_err(db_err)?;
    let Some(row) = rows.next().await.map_err(db_err)? else {
        return Ok(None);
    };
    Ok(Some(UserInfo {
        id: row.get(0).map_err(db_err)?,
        username: row.get(1).map_err(db_err)?,
        display_name: row.get(2).map_err(db_err)?,
    }))
}

pub async fn require_user(
    conn: &Connection,
    headers: &HeaderMap,
) -> Result<UserInfo, ServerFnError> {
    current_user(conn, headers)
        .await?
        .ok_or_else(|| user_err(StatusCode::UNAUTHORIZED, "Entre na sua conta primeiro."))
}

pub async fn user_id_by_username(
    conn: &Connection,
    username: &str,
) -> Result<Option<i64>, ServerFnError> {
    let mut rows = conn
        .query(
            "SELECT id FROM users WHERE username = ?1",
            params![username],
        )
        .await
        .map_err(db_err)?;
    match rows.next().await.map_err(db_err)? {
        Some(row) => Ok(Some(row.get(0).map_err(db_err)?)),
        None => Ok(None),
    }
}

pub struct RoomRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub mode: Mode,
    pub owner_id: i64,
    pub owner: String,
}

pub async fn room_by_code(conn: &Connection, code: &str) -> Result<RoomRow, ServerFnError> {
    let mut rows = conn
        .query(
            "SELECT r.id, r.code, r.name, r.mode, r.owner_id, u.display_name
             FROM rooms r JOIN users u ON u.id = r.owner_id WHERE r.code = ?1",
            params![code.trim().to_uppercase()],
        )
        .await
        .map_err(db_err)?;
    let row = rows
        .next()
        .await
        .map_err(db_err)?
        .ok_or_else(|| user_err(StatusCode::NOT_FOUND, "Sala não encontrada."))?;
    let mode: String = row.get(3).map_err(db_err)?;
    Ok(RoomRow {
        id: row.get(0).map_err(db_err)?,
        code: row.get(1).map_err(db_err)?,
        name: row.get(2).map_err(db_err)?,
        mode: Mode::parse(&mode).ok_or_else(|| db_err("modo inválido"))?,
        owner_id: row.get(4).map_err(db_err)?,
        owner: row.get(5).map_err(db_err)?,
    })
}

pub struct PlayerRow {
    pub player: Player,
    pub user_id: Option<i64>,
}

pub async fn room_players(
    conn: &Connection,
    room_id: i64,
) -> Result<Vec<PlayerRow>, ServerFnError> {
    let mut rows = conn
        .query(
            "SELECT p.id, p.name, u.username, p.user_id, p.color FROM players p
             LEFT JOIN users u ON u.id = p.user_id
             WHERE p.room_id = ?1 ORDER BY p.seat",
            params![room_id],
        )
        .await
        .map_err(db_err)?;
    let mut players = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_err)? {
        players.push(PlayerRow {
            player: Player {
                id: row.get(0).map_err(db_err)?,
                name: row.get(1).map_err(db_err)?,
                username: row.get(2).map_err(db_err)?,
                color: row.get::<i64>(4).map_err(db_err)? as u8,
            },
            user_id: row.get(3).map_err(db_err)?,
        });
    }
    Ok(players)
}

/// Bois da sala com as mãos, do mais antigo para o mais recente.
pub async fn room_bois(conn: &Connection, room_id: i64) -> Result<Vec<BoiView>, ServerFnError> {
    let mut rows = conn
        .query(
            "SELECT id, seating, started_at, colors FROM bois WHERE room_id = ?1 ORDER BY id",
            params![room_id],
        )
        .await
        .map_err(db_err)?;
    let mut bois = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_err)? {
        let seating: String = row.get(1).map_err(db_err)?;
        let colors: String = row.get(3).map_err(db_err)?;
        bois.push(BoiView {
            id: row.get(0).map_err(db_err)?,
            number: bois.len() as u32 + 1,
            seating: serde_json::from_str(&seating).map_err(db_err)?,
            hands: Vec::new(),
            colors: serde_json::from_str(&colors).map_err(db_err)?,
            started_at: row.get(2).map_err(db_err)?,
        });
    }

    let mut rows = conn
        .query(
            "SELECT m.boi_id, m.starter_seat, m.outcome, m.winner_side FROM maos m
             JOIN bois b ON b.id = m.boi_id WHERE b.room_id = ?1 ORDER BY m.boi_id, m.seq",
            params![room_id],
        )
        .await
        .map_err(db_err)?;
    while let Some(row) = rows.next().await.map_err(db_err)? {
        let boi_id: i64 = row.get(0).map_err(db_err)?;
        let outcome: String = row.get(2).map_err(db_err)?;
        let winner_side: Option<i64> = row.get(3).map_err(db_err)?;
        let hand = Hand {
            starter_seat: row.get::<i64>(1).map_err(db_err)? as usize,
            outcome: Outcome::parse(&outcome).ok_or_else(|| db_err("resultado inválido"))?,
            winner_side: winner_side.map(|s| s as usize),
        };
        if let Some(boi) = bois.iter_mut().find(|b| b.id == boi_id) {
            boi.hands.push(hand);
        }
    }
    Ok(bois)
}

pub async fn room_pairs(conn: &Connection, room_id: i64) -> Result<Vec<PairColors>, ServerFnError> {
    let mut rows = conn
        .query(
            "SELECT player_a, player_b, color, color2, chosen_at FROM pairs WHERE room_id = ?1",
            params![room_id],
        )
        .await
        .map_err(db_err)?;
    let mut pairs = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_err)? {
        pairs.push(PairColors {
            players: (row.get(0).map_err(db_err)?, row.get(1).map_err(db_err)?),
            primary: row.get::<i64>(2).map_err(db_err)? as u8,
            secondary: row.get::<i64>(3).map_err(db_err)? as u8,
            chosen_at: row.get(4).map_err(db_err)?,
        });
    }
    Ok(pairs)
}

/// Cor de cada lado para esta roda. Nas duplas, grava as duplas que ainda não
/// têm cor, para a cor ficar estável a partir daqui.
pub async fn settle_colors(
    conn: &Connection,
    room_id: i64,
    mode: Mode,
    seating: &[i64],
) -> Result<Vec<u8>, ServerFnError> {
    let players: Vec<Player> = room_players(conn, room_id)
        .await?
        .into_iter()
        .map(|p| p.player)
        .collect();
    let pairs = room_pairs(conn, room_id).await?;
    let colors = resolve_side_colors(mode, seating, &players, &pairs);
    if mode == Mode::Duplas {
        for (side, ids) in sides_of(mode, seating).iter().enumerate() {
            let [a, b] = ids[..] else { continue };
            let key = pair_key(a, b);
            if pairs.iter().any(|p| p.players == key) {
                continue;
            }
            let secondary = palette::first_free(&colors);
            conn.execute(
                "INSERT INTO pairs (room_id, player_a, player_b, color, color2) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![room_id, key.0, key.1, colors[side] as i64, secondary as i64],
            )
            .await
            .map_err(db_err)?;
        }
    }
    Ok(colors)
}

/// Recalcula as cores do boi atual, se ele ainda não acabou: quem troca de cor
/// no meio do boi vê a troca na hora. Bois encerrados mantêm as cores.
pub async fn refresh_current_boi_colors(
    conn: &Connection,
    room_id: i64,
    mode: Mode,
) -> Result<(), ServerFnError> {
    let Some(boi) = room_bois(conn, room_id).await?.pop() else {
        return Ok(());
    };
    if crate::rules::evaluate(mode, boi.seating.len(), &boi.hands).finished() {
        return Ok(());
    }
    let colors = settle_colors(conn, room_id, mode, &boi.seating).await?;
    conn.execute(
        "UPDATE bois SET colors = ?1 WHERE id = ?2",
        params![serde_json::to_string(&colors).map_err(db_err)?, boi.id],
    )
    .await
    .map_err(db_err)?;
    Ok(())
}

pub fn bois_stats<'a>(
    mode: Mode,
    bois: impl IntoIterator<Item = &'a BoiView>,
) -> Vec<crate::rules::PlayerStats> {
    crate::rules::compute_stats(
        bois.into_iter()
            .map(|b| (mode, b.seating.as_slice(), b.hands.as_slice())),
    )
}

fn room_comebacks(mode: Mode, bois: &[BoiView]) -> Vec<crate::rules::Comeback> {
    let finished = bois
        .iter()
        .filter(|b| crate::rules::evaluate(mode, b.seating.len(), &b.hands).finished())
        .count();
    if finished < crate::rules::MIN_BOIS_FOR_COMEBACKS {
        return Vec::new();
    }
    crate::rules::comebacks(mode, bois)
}

/// Dias com pelo menos uma mão, do mais recente para o mais antigo.
fn played_days(bois: &[BoiView]) -> Vec<Period> {
    let mut days: Vec<Period> = Vec::new();
    for boi in bois.iter().rev().filter(|b| !b.hands.is_empty()) {
        let day = Period::day_of(boi.started_at);
        if !days.contains(&day) {
            days.push(day);
        }
    }
    days
}

pub async fn load_room_view(
    conn: &Connection,
    room: &RoomRow,
    user: Option<&UserInfo>,
) -> Result<RoomView, ServerFnError> {
    let players = room_players(conn, room.id).await?;
    let bois = room_bois(conn, room.id).await?;
    let is_owner = user.is_some_and(|u| u.id == room.owner_id);
    let can_edit =
        is_owner || user.is_some_and(|u| players.iter().any(|p| p.user_id == Some(u.id)));
    Ok(RoomView {
        code: room.code.clone(),
        name: room.name.clone(),
        mode: room.mode,
        owner: room.owner.clone(),
        is_owner,
        can_edit,
        players: players.into_iter().map(|p| p.player).collect(),
        current: bois.last().cloned(),
        history: history_page(room.mode, &bois, None),
        stats: bois_stats(room.mode, bois.iter()),
        played: played_days(&bois),
        comebacks: room_comebacks(room.mode, &bois),
        pairs: room_pairs(conn, room.id).await?,
    })
}

/// Sala + verificação de que o usuário pode registrar mãos nela.
pub async fn editable_room(
    conn: &Connection,
    headers: &HeaderMap,
    code: &str,
) -> Result<(RoomRow, RoomView), ServerFnError> {
    let user = require_user(conn, headers).await?;
    let room = room_by_code(conn, code).await?;
    let view = load_room_view(conn, &room, Some(&user)).await?;
    if !view.can_edit {
        return Err(user_err(
            StatusCode::FORBIDDEN,
            "Só o dono da sala e os jogadores com conta podem marcar o placar.",
        ));
    }
    Ok((room, view))
}

// ---- Tempo real ----
//
// Cada sala tem um canal. Quem altera a sala avisa o canal, e quem está assistindo
// recebe o aviso por SSE e recarrega o placar.

static CHANNELS: LazyLock<Mutex<HashMap<i64, broadcast::Sender<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn subscribe(room_id: i64) -> broadcast::Receiver<()> {
    let mut channels = CHANNELS.lock().unwrap();
    channels
        .entry(room_id)
        .or_insert_with(|| broadcast::channel(16).0)
        .subscribe()
}

pub fn notify(room_id: i64) {
    let mut channels = CHANNELS.lock().unwrap();
    if let Some(tx) = channels.get(&room_id)
        && tx.send(()).is_err()
    {
        // Ninguém assistindo.
        channels.remove(&room_id);
    }
}
