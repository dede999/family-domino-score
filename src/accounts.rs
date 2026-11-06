//! Contas: senhas, sessões e quem está logado. Só no servidor.

use crate::server::{db_err, random_token, user_err};
use crate::views::UserInfo;
use dioxus::fullstack::{HeaderMap, ServerFnError, StatusCode};
use libsql::{Connection, params};

pub const SESSION_COOKIE: &str = "placar_session";

/// Código por e-mail vale 15 minutos.
pub const CODE_TTL: i64 = 15 * 60;
/// Link gerado pelo administrador vale 1 hora.
pub const LINK_TTL: i64 = 60 * 60;
/// Erros permitidos por código antes de ele deixar de valer.
pub const MAX_ATTEMPTS: i64 = 5;
/// Intervalo mínimo entre dois códigos pedidos para a mesma conta.
pub const RESEND_AFTER: i64 = 60;

// ---- Regras (sem banco) ----

/// Nome de usuário: 3 ou mais letras, números, `_` ou `.`, sem maiúsculas.
pub fn check_username(username: &str) -> Result<String, &'static str> {
    let u = username.trim().to_lowercase();
    if u.len() < 3
        || !u
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return Err("O usuário precisa ter 3 ou mais letras ou números, sem espaços.");
    }
    Ok(u)
}

pub fn check_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 6 {
        return Err("A senha precisa ter pelo menos 6 caracteres.");
    }
    Ok(())
}

/// E-mail sem espaços, em minúsculas, com um `@` e um ponto depois dele.
/// Não tenta ser completo: quem confirma de verdade é o código.
pub fn check_email(email: &str) -> Result<String, &'static str> {
    let e = email.trim().to_lowercase();
    let ok = match e.split_once('@') {
        Some((user, domain)) => {
            !user.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !e.chars().any(char::is_whitespace)
        }
        None => false,
    };
    if !ok {
        return Err("Esse e-mail não parece certo.");
    }
    Ok(e)
}

/// Estado de um código guardado, na hora de conferir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeState {
    Usable,
    Expired,
    TooManyAttempts,
}

pub fn code_state(expires_at: i64, attempts: i64, now: i64) -> CodeState {
    if now >= expires_at {
        CodeState::Expired
    } else if attempts >= MAX_ATTEMPTS {
        CodeState::TooManyAttempts
    } else {
        CodeState::Usable
    }
}

/// Código de 6 dígitos para mandar por e-mail.
pub fn new_code() -> String {
    random_token(6, b"0123456789")
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
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

/// Cookie da sessão. Com `PLACAR_HTTPS` definida (produção), só viaja por
/// HTTPS; em casa, por HTTP, o navegador não guardaria um cookie `Secure`.
pub fn session_cookie(token: &str, max_age: u32) -> String {
    let secure = if std::env::var_os("PLACAR_HTTPS").is_some() {
        "; Secure"
    } else {
        ""
    };
    format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}")
}

pub fn session_token(headers: &HeaderMap) -> Option<String> {
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

// ---- Exclusão da conta ----

/// Apaga a conta e o que é só dela: sessões, códigos, e-mail e senha.
///
/// - Nas salas em que ela joga, o jogador fica com o mesmo nome, sem conta:
///   o histórico da sala é de todos que jogaram.
/// - As salas que ela criou passam para o primeiro jogador com conta, pela
///   ordem da roda. Sem nenhum, a sala é apagada com o histórico.
///
/// Devolve as salas que mudaram, para avisar quem está assistindo.
pub async fn delete_account(conn: &Connection, user_id: i64) -> Result<Vec<i64>, ServerFnError> {
    async fn ids(
        conn: &Connection,
        sql: &str,
        params: impl libsql::params::IntoParams,
    ) -> Result<Vec<i64>, ServerFnError> {
        let mut rows = conn.query(sql, params).await.map_err(db_err)?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next().await.map_err(db_err)? {
            ids.push(row.get(0).map_err(db_err)?);
        }
        Ok(ids)
    }

    let tx = conn.transaction().await.map_err(db_err)?;
    let mut changed = ids(
        &tx,
        "SELECT DISTINCT room_id FROM players WHERE user_id = ?1",
        params![user_id],
    )
    .await?;
    for room in ids(
        &tx,
        "SELECT id FROM rooms WHERE owner_id = ?1",
        params![user_id],
    )
    .await?
    {
        let heir = ids(
            &tx,
            "SELECT user_id FROM players
             WHERE room_id = ?1 AND user_id IS NOT NULL AND user_id <> ?2
             ORDER BY seat LIMIT 1",
            params![room, user_id],
        )
        .await?;
        if let Some(heir) = heir.first() {
            tx.execute(
                "UPDATE rooms SET owner_id = ?1 WHERE id = ?2",
                params![*heir, room],
            )
            .await
            .map_err(db_err)?;
            if !changed.contains(&room) {
                changed.push(room);
            }
        } else {
            tx.execute_batch(&format!(
                "DELETE FROM pairs WHERE room_id = {room};
                 DELETE FROM maos WHERE boi_id IN (SELECT id FROM bois WHERE room_id = {room});
                 DELETE FROM bois WHERE room_id = {room};
                 DELETE FROM players WHERE room_id = {room};
                 DELETE FROM rooms WHERE id = {room};"
            ))
            .await
            .map_err(db_err)?;
        }
    }
    // Os ids são números, então dá para pô-los direto no SQL.
    tx.execute_batch(&format!(
        "UPDATE players SET user_id = NULL WHERE user_id = {user_id};
         DELETE FROM codes WHERE user_id = {user_id};
         DELETE FROM sessions WHERE user_id = {user_id};
         DELETE FROM users WHERE id = {user_id};"
    ))
    .await
    .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(changed)
}

// ---- Códigos de uso único ----

/// Guarda um código novo (só o hash) e apaga os anteriores do mesmo tipo.
pub async fn store_code(
    conn: &Connection,
    user_id: i64,
    purpose: &str,
    secret: &str,
    email: Option<&str>,
    ttl: i64,
) -> Result<(), ServerFnError> {
    conn.execute(
        "DELETE FROM codes WHERE user_id = ?1 AND purpose = ?2",
        params![user_id, purpose],
    )
    .await
    .map_err(db_err)?;
    conn.execute(
        "INSERT INTO codes (user_id, purpose, code_hash, email, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            user_id,
            purpose,
            hash_password(secret)?,
            email.map(str::to_string),
            now() + ttl
        ],
    )
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Apaga o código de um tipo, para quem não recebeu o e-mail poder pedir
/// outro na hora (sem esperar o minuto de `recently_sent`).
pub async fn forget_code(
    conn: &Connection,
    user_id: i64,
    purpose: &str,
) -> Result<(), ServerFnError> {
    conn.execute(
        "DELETE FROM codes WHERE user_id = ?1 AND purpose = ?2",
        params![user_id, purpose],
    )
    .await
    .map_err(db_err)?;
    Ok(())
}

/// Pediu um código há pouco? Evita encher a caixa de alguém de e-mails.
pub async fn recently_sent(
    conn: &Connection,
    user_id: i64,
    purpose: &str,
) -> Result<bool, ServerFnError> {
    let mut rows = conn
        .query(
            "SELECT 1 FROM codes WHERE user_id = ?1 AND purpose = ?2 AND created_at > ?3",
            params![user_id, purpose, now() - RESEND_AFTER],
        )
        .await
        .map_err(db_err)?;
    Ok(rows.next().await.map_err(db_err)?.is_some())
}

/// Confere um código. Se servir, apaga os códigos desses tipos e devolve o
/// e-mail guardado com ele (só os de confirmar e-mail têm). Se não servir,
/// conta a tentativa.
pub async fn redeem_code(
    conn: &Connection,
    user_id: i64,
    purposes: &[&str],
    given: &str,
) -> Result<Option<String>, ServerFnError> {
    let wrong = || {
        user_err(
            StatusCode::BAD_REQUEST,
            "Código errado ou vencido. Confira ou peça outro.",
        )
    };
    let mut found = None;
    for purpose in purposes {
        let mut rows = conn
            .query(
                "SELECT id, code_hash, email, attempts, expires_at FROM codes
                 WHERE user_id = ?1 AND purpose = ?2",
                params![user_id, *purpose],
            )
            .await
            .map_err(db_err)?;
        while let Some(row) = rows.next().await.map_err(db_err)? {
            let id: i64 = row.get(0).map_err(db_err)?;
            let hash: String = row.get(1).map_err(db_err)?;
            let email: Option<String> = row.get(2).map_err(db_err)?;
            let attempts: i64 = row.get(3).map_err(db_err)?;
            let expires_at: i64 = row.get(4).map_err(db_err)?;
            match code_state(expires_at, attempts, now()) {
                CodeState::Usable if verify_password(given.trim(), &hash) => {
                    found = Some(Ok(email));
                }
                CodeState::Usable => {
                    conn.execute(
                        "UPDATE codes SET attempts = attempts + 1 WHERE id = ?1",
                        params![id],
                    )
                    .await
                    .map_err(db_err)?;
                }
                CodeState::TooManyAttempts if found.is_none() => {
                    found = Some(Err(user_err(
                        StatusCode::BAD_REQUEST,
                        "Muitas tentativas erradas. Peça um código novo.",
                    )));
                }
                _ => {}
            }
        }
    }
    match found {
        Some(Ok(email)) => {
            for purpose in purposes {
                conn.execute(
                    "DELETE FROM codes WHERE user_id = ?1 AND purpose = ?2",
                    params![user_id, *purpose],
                )
                .await
                .map_err(db_err)?;
            }
            Ok(email)
        }
        Some(Err(e)) => Err(e),
        None => Err(wrong()),
    }
}

/// Conta pelo nome de usuário ou pelo e-mail: (usuário, e-mail confirmado).
pub async fn find_account(
    conn: &Connection,
    who: &str,
) -> Result<Option<(UserInfo, Option<String>)>, ServerFnError> {
    let who = who.trim().to_lowercase();
    let mut rows = conn
        .query(
            "SELECT id, username, display_name, email FROM users
             WHERE username = ?1 OR email = ?1",
            params![who],
        )
        .await
        .map_err(db_err)?;
    let Some(row) = rows.next().await.map_err(db_err)? else {
        return Ok(None);
    };
    Ok(Some((
        UserInfo {
            id: row.get(0).map_err(db_err)?,
            username: row.get(1).map_err(db_err)?,
            display_name: row.get(2).map_err(db_err)?,
        },
        row.get(3).map_err(db_err)?,
    )))
}

/// Administrador: a conta cujo usuário está em `PLACAR_ADMIN`.
pub fn is_admin(user: &UserInfo) -> bool {
    std::env::var("PLACAR_ADMIN").is_ok_and(|a| a.trim().eq_ignore_ascii_case(&user.username))
}

// ---- E-mail ----

/// Manda um e-mail. Com `PLACAR_SMTP_USER` e `PLACAR_SMTP_PASSWORD` (a conta
/// do Gmail do app e uma senha de app dela), envia pelo SMTP do Gmail, ou do
/// servidor em `PLACAR_SMTP_HOST`. Sem elas (desenvolvimento), só escreve no
/// log e, com `PLACAR_MAILBOX`, num arquivo que o teste da interface lê.
///
/// No envio de verdade, o log não leva o corpo: ele tem o código.
pub async fn send_email(to: &str, subject: &str, body: &str) -> Result<(), String> {
    let (Ok(user), Ok(password)) = (
        std::env::var("PLACAR_SMTP_USER"),
        std::env::var("PLACAR_SMTP_PASSWORD"),
    ) else {
        write_mailbox(to, subject, body);
        return Ok(());
    };
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let host = std::env::var("PLACAR_SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".into());
    let from = format!("Placar de Dominó <{user}>")
        .parse()
        .map_err(|e| format!("remetente inválido: {e}"))?;
    let to_addr = to
        .parse()
        .map_err(|e| format!("destinatário inválido: {e}"))?;
    let message = Message::builder()
        .from(from)
        .to(to_addr)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| e.to_string())?;
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
        .map_err(|e| e.to_string())?
        .credentials(Credentials::new(user, password))
        .build();
    mailer.send(message).await.map_err(|e| e.to_string())?;
    eprintln!("e-mail enviado para {to}: {subject}");
    Ok(())
}

fn write_mailbox(to: &str, subject: &str, body: &str) {
    let line = format!("para {to} | {subject} | {body}");
    eprintln!("e-mail (não enviado, sem SMTP) {line}");
    if let Ok(path) = std::env::var("PLACAR_MAILBOX") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usuario_senha_e_email() {
        assert_eq!(check_username(" Andre.S "), Ok("andre.s".into()));
        assert!(check_username("ab").is_err());
        assert!(check_username("com espaço").is_err());
        assert!(check_password("12345").is_err());
        assert!(check_password("123456").is_ok());
        assert_eq!(
            check_email(" Andre@Exemplo.com "),
            Ok("andre@exemplo.com".into())
        );
        for ruim in [
            "andre",
            "@x.com",
            "a@b",
            "a@b.",
            "a@.com",
            "a b@c.com",
            "a@b@c.com",
        ] {
            assert!(check_email(ruim).is_err(), "{ruim}");
        }
    }

    #[test]
    fn codigo_vence_e_tem_limite_de_tentativas() {
        assert_eq!(code_state(100, 0, 99), CodeState::Usable);
        assert_eq!(code_state(100, 0, 100), CodeState::Expired);
        assert_eq!(
            code_state(100, MAX_ATTEMPTS, 50),
            CodeState::TooManyAttempts
        );
        assert_eq!(code_state(100, MAX_ATTEMPTS - 1, 50), CodeState::Usable);
        let c = new_code();
        assert!(c.len() == 6 && c.chars().all(|c| c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn excluir_conta_passa_a_sala_ou_apaga() {
        let (_db, conn) = crate::server::test_db().await;
        // 1 = quem sai, 2 = outro com conta. Sala 10: de 1, com 2 jogando
        // (passa para 2). Sala 20: de 1, só avulsos (apagada). Sala 30: de 2,
        // com 1 jogando (fica, com 1 avulso).
        conn.execute_batch(
            "INSERT INTO users (id, username, display_name, password_hash) VALUES
                 (1, 'sai', 'Sai', 'x'), (2, 'fica', 'Fica', 'x');
             INSERT INTO sessions (token, user_id) VALUES ('t1', 1), ('t2', 2);
             INSERT INTO codes (user_id, purpose, code_hash, expires_at) VALUES (1, 'login', 'h', 0);
             INSERT INTO rooms (id, code, name, mode, owner_id) VALUES
                 (10, 'AAA', 'a', 'individual', 1),
                 (20, 'BBB', 'b', 'individual', 1),
                 (30, 'CCC', 'c', 'individual', 2);
             INSERT INTO players (id, room_id, name, user_id, seat) VALUES
                 (1, 10, 'Sai', 1, 0), (2, 10, 'Avulso', NULL, 1), (3, 10, 'Fica', 2, 2),
                 (4, 20, 'Sai', 1, 0), (5, 20, 'Avulso', NULL, 1),
                 (6, 30, 'Fica', 2, 0), (7, 30, 'Sai', 1, 1);
             INSERT INTO bois (id, room_id, seating) VALUES (1, 20, '[4,5]'), (2, 30, '[6,7]');
             INSERT INTO maos (boi_id, seq, starter_seat, outcome, winner_side) VALUES
                 (1, 1, 0, 'batida', 0), (2, 1, 0, 'batida', 1);",
        )
        .await
        .unwrap();

        let mut changed = delete_account(&conn, 1).await.unwrap();
        changed.sort();
        assert_eq!(changed, vec![10, 20, 30]);

        let count = |sql: &'static str| {
            let conn = conn.clone();
            async move {
                let mut rows = conn.query(sql, ()).await.unwrap();
                rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
            }
        };
        assert_eq!(count("SELECT count(*) FROM users WHERE id = 1").await, 0);
        assert_eq!(count("SELECT count(*) FROM sessions").await, 1);
        assert_eq!(count("SELECT count(*) FROM codes").await, 0);
        assert_eq!(count("SELECT owner_id FROM rooms WHERE id = 10").await, 2);
        assert_eq!(count("SELECT count(*) FROM rooms WHERE id = 20").await, 0);
        assert_eq!(
            count("SELECT count(*) FROM players WHERE room_id = 20").await,
            0
        );
        assert_eq!(count("SELECT count(*) FROM maos WHERE boi_id = 1").await, 0);
        // Na sala dos outros, o nome e as mãos continuam, sem a conta.
        assert_eq!(
            count("SELECT count(*) FROM players WHERE id = 7 AND name = 'Sai' AND user_id IS NULL")
                .await,
            1
        );
        assert_eq!(count("SELECT count(*) FROM maos WHERE boi_id = 2").await, 1);
    }
}
