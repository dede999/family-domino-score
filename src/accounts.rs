//! Contas: senhas, sessões e quem está logado. Só no servidor.

use crate::model::UserInfo;
use crate::server::{db_err, random_token, user_err};
use dioxus::fullstack::{HeaderMap, ServerFnError, StatusCode};
use libsql::{Connection, params};

pub const SESSION_COOKIE: &str = "placar_session";

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
