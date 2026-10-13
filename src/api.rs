//! Funções de servidor. No navegador, cada uma vira uma chamada HTTP.

use crate::model::{
    HistoryPage, Mode, Outcome, Period, PeriodReport, PlayerInput, RoomSummary, RoomView, UserInfo,
};
use crate::rules::PlayerStats;
use dioxus::fullstack::{Json, ServerEvents, SetCookie, SetHeader};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::room_rules::{self, Undo};
#[cfg(feature = "server")]
use crate::server::*;
#[cfg(feature = "server")]
use dioxus::fullstack::HeaderMap;
#[cfg(feature = "server")]
use libsql::params;

/// Mensagem de erro para mostrar na tela.
pub fn error_message(err: &ServerFnError) -> String {
    match err {
        // A mensagem original vem em `details`; `message` chega com prefixo técnico.
        ServerFnError::ServerError {
            message, details, ..
        } => details
            .as_ref()
            .and_then(|d| d.get("ServerError")?.get("message")?.as_str())
            .map(String::from)
            .unwrap_or_else(|| message.clone()),
        ServerFnError::Request(_) => "Sem conexão com o servidor.".into(),
        other => other.to_string(),
    }
}

type Login = (SetHeader<SetCookie>, Json<UserInfo>);

#[cfg(feature = "server")]
async fn start_session(conn: &libsql::Connection, user: UserInfo) -> Result<Login, ServerFnError> {
    let token = create_session(conn, user.id).await?;
    let cookie = SetHeader::new(session_cookie(&token, 60 * 60 * 24 * 365)).map_err(db_err)?;
    Ok((cookie, Json(user)))
}

#[get("/api/me", headers: HeaderMap)]
pub async fn me() -> Result<Option<UserInfo>, ServerFnError> {
    let conn = conn().await?;
    current_user(&conn, &headers).await
}

#[post("/api/cadastro")]
pub async fn register(
    username: String,
    display_name: String,
    password: String,
) -> Result<Login, ServerFnError> {
    let username = username.trim().to_lowercase();
    let display_name = display_name.trim().to_string();
    if username.len() < 3
        || !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return Err(user_err(
            StatusCode::BAD_REQUEST,
            "O usuário precisa ter 3 ou mais letras ou números, sem espaços.",
        ));
    }
    if display_name.is_empty() {
        return Err(user_err(StatusCode::BAD_REQUEST, "Informe seu nome."));
    }
    if password.len() < 6 {
        return Err(user_err(
            StatusCode::BAD_REQUEST,
            "A senha precisa ter pelo menos 6 caracteres.",
        ));
    }
    let conn = conn().await?;
    if user_id_by_username(&conn, &username).await?.is_some() {
        return Err(user_err(StatusCode::CONFLICT, "Esse usuário já existe."));
    }
    let hash = hash_password(&password)?;
    conn.execute(
        "INSERT INTO users (username, display_name, password_hash) VALUES (?1, ?2, ?3)",
        params![username.clone(), display_name.clone(), hash],
    )
    .await
    .map_err(db_err)?;
    let user = UserInfo {
        id: conn.last_insert_rowid(),
        username,
        display_name,
    };
    start_session(&conn, user).await
}

#[post("/api/entrar")]
pub async fn login(username: String, password: String) -> Result<Login, ServerFnError> {
    let conn = conn().await?;
    let mut rows = conn
        .query(
            "SELECT id, username, display_name, password_hash FROM users WHERE username = ?1",
            params![username.trim().to_lowercase()],
        )
        .await
        .map_err(db_err)?;
    let wrong = || user_err(StatusCode::UNAUTHORIZED, "Usuário ou senha incorretos.");
    let row = rows.next().await.map_err(db_err)?.ok_or_else(wrong)?;
    let hash: String = row.get(3).map_err(db_err)?;
    if !verify_password(&password, &hash) {
        return Err(wrong());
    }
    let user = UserInfo {
        id: row.get(0).map_err(db_err)?,
        username: row.get(1).map_err(db_err)?,
        display_name: row.get(2).map_err(db_err)?,
    };
    start_session(&conn, user).await
}

#[post("/api/sair", headers: HeaderMap)]
pub async fn logout() -> Result<SetHeader<SetCookie>, ServerFnError> {
    let conn = conn().await?;
    if let Some(user) = current_user(&conn, &headers).await? {
        conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user.id])
            .await
            .map_err(db_err)?;
    }
    SetHeader::new(session_cookie("", 0)).map_err(db_err)
}

/// Salas em que o usuário é dono ou jogador.
#[get("/api/minhas-salas", headers: HeaderMap)]
pub async fn my_rooms() -> Result<Vec<RoomSummary>, ServerFnError> {
    let conn = conn().await?;
    let user = require_user(&conn, &headers).await?;
    let mut rows = conn
        .query(
            "SELECT DISTINCT r.code, r.name, r.mode FROM rooms r
             LEFT JOIN players p ON p.room_id = r.id
             WHERE r.owner_id = ?1 OR p.user_id = ?1 ORDER BY r.id DESC",
            params![user.id],
        )
        .await
        .map_err(db_err)?;
    let mut rooms = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_err)? {
        let mode: String = row.get(2).map_err(db_err)?;
        rooms.push(RoomSummary {
            code: row.get(0).map_err(db_err)?,
            name: row.get(1).map_err(db_err)?,
            mode: Mode::parse(&mode).unwrap_or(Mode::Individual),
        });
    }
    Ok(rooms)
}

/// Estatísticas do usuário somadas em todas as salas.
#[get("/api/minhas-estatisticas", headers: HeaderMap)]
pub async fn my_stats() -> Result<PlayerStats, ServerFnError> {
    let conn = conn().await?;
    let user = require_user(&conn, &headers).await?;
    let mut rows = conn
        .query(
            "SELECT DISTINCT room_id FROM players WHERE user_id = ?1",
            params![user.id],
        )
        .await
        .map_err(db_err)?;
    let mut room_ids = Vec::new();
    while let Some(row) = rows.next().await.map_err(db_err)? {
        room_ids.push(row.get::<i64>(0).map_err(db_err)?);
    }
    let mut total = PlayerStats::default();
    for room_id in room_ids {
        let players = room_players(&conn, room_id).await?;
        let mine: Vec<i64> = players
            .iter()
            .filter(|p| p.user_id == Some(user.id))
            .map(|p| p.player.id)
            .collect();
        let mut rows = conn
            .query("SELECT mode FROM rooms WHERE id = ?1", params![room_id])
            .await
            .map_err(db_err)?;
        let Some(row) = rows.next().await.map_err(db_err)? else {
            continue;
        };
        let mode = Mode::parse(&row.get::<String>(0).map_err(db_err)?).unwrap_or(Mode::Individual);
        let bois = room_bois(&conn, room_id).await?;
        let stats = bois_stats(mode, &bois);
        for s in stats.iter().filter(|s| mine.contains(&s.player_id)) {
            total.add(s);
        }
    }
    Ok(total)
}

/// Resolve o usuário vinculado de cada jogador.
#[cfg(feature = "server")]
async fn resolve_links(
    conn: &libsql::Connection,
    players: &[PlayerInput],
) -> Result<Vec<Option<i64>>, ServerFnError> {
    let mut links = Vec::new();
    for p in players {
        let username = p
            .username
            .as_deref()
            .map(str::trim)
            .filter(|u| !u.is_empty());
        let link = match username {
            Some(u) => Some(
                user_id_by_username(conn, &u.to_lowercase())
                    .await?
                    .ok_or_else(|| {
                        user_err(
                            StatusCode::BAD_REQUEST,
                            format!("Usuário \"{u}\" não existe."),
                        )
                    })?,
            ),
            None => None,
        };
        links.push(link);
    }
    room_rules::distinct_links(&links)?;
    Ok(links)
}

/// Cria a sala. A lista de jogadores vem na ordem da roda.
#[post("/api/salas", headers: HeaderMap)]
pub async fn create_room(
    name: String,
    mode: Mode,
    players: Vec<PlayerInput>,
) -> Result<String, ServerFnError> {
    let conn = conn().await?;
    let user = require_user(&conn, &headers).await?;
    let name = room_rules::room_name(&name)?;
    room_rules::validate_players(mode, &players)?;
    let links = resolve_links(&conn, &players).await?;
    let colors = room_rules::assign_colors(&players)?;

    // Sem 0/O e 1/I para facilitar ditar o código.
    let code = random_token(6, b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789");
    let tx = conn.transaction().await.map_err(db_err)?;
    tx.execute(
        "INSERT INTO rooms (code, name, mode, owner_id) VALUES (?1, ?2, ?3, ?4)",
        params![code.clone(), name, mode.as_str(), user.id],
    )
    .await
    .map_err(db_err)?;
    let room_id = tx.last_insert_rowid();
    for (seat, ((p, link), color)) in players.iter().zip(links).zip(colors).enumerate() {
        tx.execute(
            "INSERT INTO players (room_id, name, user_id, seat, color) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                room_id,
                p.name.trim().to_string(),
                link,
                seat as i64,
                color as i64
            ],
        )
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(code)
}

#[get("/api/salas/{code}", headers: HeaderMap)]
pub async fn get_room(code: String) -> Result<RoomView, ServerFnError> {
    let conn = conn().await?;
    let user = current_user(&conn, &headers).await?;
    let room = room_by_code(&conn, &code).await?;
    load_room_view(&conn, &room, user.as_ref()).await
}

/// Próxima página do histórico: bois terminados mais antigos que `before`.
#[post("/api/salas/{code}/historico")]
pub async fn room_history(code: String, before: i64) -> Result<HistoryPage, ServerFnError> {
    let conn = conn().await?;
    let room = room_by_code(&conn, &code).await?;
    let bois = room_bois(&conn, room.id).await?;
    Ok(crate::model::history_page(room.mode, &bois, Some(before)))
}

/// Estatísticas da sala em cada período pedido (no máximo dois, para
/// comparar), e as primeiras vezes quando o período é um ano. Um boi conta
/// no período do dia em que começou.
#[post("/api/salas/{code}/estatisticas")]
pub async fn room_stats(
    code: String,
    periods: Vec<Period>,
) -> Result<Vec<PeriodReport>, ServerFnError> {
    if periods.len() > 2 {
        return Err(user_err(
            StatusCode::BAD_REQUEST,
            "No máximo dois períodos.",
        ));
    }
    let conn = conn().await?;
    let room = room_by_code(&conn, &code).await?;
    let bois = room_bois(&conn, room.id).await?;
    Ok(periods
        .iter()
        .map(|&p| {
            let inside = || bois.iter().filter(move |b| p.contains(b.started_at));
            PeriodReport {
                stats: bois_stats(room.mode, inside()),
                pairs: crate::rules::pair_records(room.mode, inside()),
                firsts: matches!(p, Period::Year { .. })
                    .then(|| crate::rules::firsts(room.mode, inside())),
            }
        })
        .collect())
}

/// Começa um boi novo com a ordem atual da roda.
#[post("/api/salas/{code}/boi", headers: HeaderMap)]
pub async fn start_boi(code: String) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let (room, view) = editable_room(&conn, &headers, &code).await?;
    room_rules::can_start_boi(room.mode, view.current())?;
    let seating: Vec<i64> = view.players.iter().map(|p| p.id).collect();
    let colors = settle_colors(&conn, room.id, room.mode, &seating).await?;
    conn.execute(
        "INSERT INTO bois (room_id, seating, colors) VALUES (?1, ?2, ?3)",
        params![
            room.id,
            serde_json::to_string(&seating).map_err(db_err)?,
            serde_json::to_string(&colors).map_err(db_err)?
        ],
    )
    .await
    .map_err(db_err)?;
    notify(room.id);
    Ok(())
}

/// Registra uma mão no boi atual. `expected_hands` evita gravar duas vezes
/// quando duas pessoas marcam ao mesmo tempo ou o botão é tocado duas vezes.
#[post("/api/salas/{code}/maos", headers: HeaderMap)]
pub async fn record_hand(
    code: String,
    expected_hands: usize,
    starter_seat: usize,
    outcome: Outcome,
    winner_side: Option<usize>,
) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let (room, view) = editable_room(&conn, &headers, &code).await?;
    let (boi_id, seq) = room_rules::place_hand(
        room.mode,
        view.current(),
        expected_hands,
        starter_seat,
        outcome,
        winner_side,
    )?;
    conn.execute(
        "INSERT INTO maos (boi_id, seq, starter_seat, outcome, winner_side)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            boi_id,
            seq as i64,
            starter_seat as i64,
            outcome.as_str(),
            winner_side.map(|s| s as i64)
        ],
    )
    .await
    .map_err(|e| {
        // A restrição UNIQUE (boi_id, seq) pega a corrida entre duas gravações.
        if e.to_string().contains("UNIQUE") {
            user_err(StatusCode::CONFLICT, "Essa mão já foi registrada.")
        } else {
            db_err(e)
        }
    })?;
    notify(room.id);
    Ok(())
}

/// Desfaz a última mão. Se o boi atual ainda não tem mãos, apaga o boi e volta ao anterior.
#[post("/api/salas/{code}/desfazer", headers: HeaderMap)]
pub async fn undo(code: String, expected_hands: usize) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let (room, view) = editable_room(&conn, &headers, &code).await?;
    match room_rules::undo(view.current(), expected_hands)? {
        Undo::Boi { boi_id } => {
            conn.execute("DELETE FROM bois WHERE id = ?1", params![boi_id])
                .await
                .map_err(db_err)?;
        }
        Undo::Hand { boi_id, seq } => {
            conn.execute(
                "DELETE FROM maos WHERE boi_id = ?1 AND seq = ?2",
                params![boi_id, seq as i64],
            )
            .await
            .map_err(db_err)?;
        }
    }
    notify(room.id);
    Ok(())
}

/// Edita os jogadores: nomes, contas vinculadas, ordem da roda e novos jogadores.
/// Só o dono pode, e só fora de um boi em andamento.
#[post("/api/salas/{code}/jogadores", headers: HeaderMap)]
pub async fn update_players(code: String, players: Vec<PlayerInput>) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let (room, view) = editable_room(&conn, &headers, &code).await?;
    let bois = room_bois(&conn, room.id).await?;
    let removed = room_rules::players_to_remove(
        view.is_owner,
        view.boi_in_progress(),
        &view.players,
        &players,
        &bois,
    )?;
    room_rules::validate_players(room.mode, &players)?;
    let links = resolve_links(&conn, &players).await?;
    let colors = room_rules::assign_colors(&players)?;

    let tx = conn.transaction().await.map_err(db_err)?;
    for id in removed {
        tx.execute(
            "DELETE FROM pairs WHERE player_a = ?1 OR player_b = ?1",
            params![id],
        )
        .await
        .map_err(db_err)?;
        tx.execute("DELETE FROM players WHERE id = ?1", params![id])
            .await
            .map_err(db_err)?;
    }
    for (seat, ((p, link), color)) in players.iter().zip(links).zip(colors).enumerate() {
        let name = p.name.trim().to_string();
        let color = color as i64;
        // Ids desconhecidos já foram recusados em `players_to_remove`.
        match p.id {
            Some(id) => {
                tx.execute(
                    "UPDATE players SET name = ?1, user_id = ?2, seat = ?3, color = ?4 WHERE id = ?5",
                    params![name, link, seat as i64, color, id],
                )
                .await
                .map_err(db_err)?;
            }
            None => {
                tx.execute(
                    "INSERT INTO players (room_id, name, user_id, seat, color) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![room.id, name, link, seat as i64, color],
                )
                .await
                .map_err(db_err)?;
            }
        }
    }
    // Um boi novo e vazio ainda usa a ordem antiga; atualiza para a nova.
    if let Some(current) = view.current().filter(|b| b.hands.is_empty()) {
        let seating: Vec<i64> = {
            let mut rows = tx
                .query(
                    "SELECT id FROM players WHERE room_id = ?1 ORDER BY seat",
                    params![room.id],
                )
                .await
                .map_err(db_err)?;
            let mut ids = Vec::new();
            while let Some(row) = rows.next().await.map_err(db_err)? {
                ids.push(row.get::<i64>(0).map_err(db_err)?);
            }
            ids
        };
        tx.execute(
            "UPDATE bois SET seating = ?1 WHERE id = ?2",
            params![serde_json::to_string(&seating).map_err(db_err)?, current.id],
        )
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    refresh_current_boi_colors(&conn, room.id, room.mode).await?;
    notify(room.id);
    Ok(())
}

/// Troca as cores de uma dupla. Qualquer pessoa que pode marcar o placar pode
/// trocar. Vale para o boi atual (se não acabou) e para os próximos.
#[post("/api/salas/{code}/duplas", headers: HeaderMap)]
pub async fn set_pair_colors(
    code: String,
    players: (i64, i64),
    primary: u8,
    secondary: u8,
) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let (room, view) = editable_room(&conn, &headers, &code).await?;
    let key = room_rules::pair_colors(room.mode, &view.players, players, primary, secondary)?;
    // `chosen_at` só muda quando a principal muda: ela decide quem cede a cor.
    conn.execute(
        "INSERT INTO pairs (room_id, player_a, player_b, color, color2) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (room_id, player_a, player_b) DO UPDATE SET
             chosen_at = CASE WHEN color = excluded.color THEN chosen_at ELSE unixepoch() END,
             color = excluded.color,
             color2 = excluded.color2",
        params![room.id, key.0, key.1, primary as i64, secondary as i64],
    )
    .await
    .map_err(db_err)?;
    refresh_current_boi_colors(&conn, room.id, room.mode).await?;
    notify(room.id);
    Ok(())
}

/// Avisa quem está assistindo a sala sempre que o placar muda.
#[get("/api/salas/{code}/eventos")]
pub async fn room_events(code: String) -> Result<ServerEvents<u8>, ServerFnError> {
    let conn = conn().await?;
    let room = room_by_code(&conn, &code).await?;
    let mut rx = subscribe(room.id);
    Ok(ServerEvents::new(move |mut tx| async move {
        use tokio::sync::broadcast::error::RecvError;
        // Para quando o canal fecha ou quem assistia desconecta.
        while let Ok(()) | Err(RecvError::Lagged(_)) = rx.recv().await {
            if tx.send(1).await.is_err() {
                break;
            }
        }
    }))
}
