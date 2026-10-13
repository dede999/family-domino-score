//! Cadastro, login e sessão.

use crate::model::UserInfo;
use dioxus::fullstack::{Json, SetCookie, SetHeader};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::accounts::*;
#[cfg(feature = "server")]
use crate::server::{conn, db_err, user_err};
#[cfg(feature = "server")]
use dioxus::fullstack::HeaderMap;
#[cfg(feature = "server")]
use libsql::params;

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
