//! Cadastro, login e sessão.

use crate::views::{Account, UserInfo};
use dioxus::fullstack::{Json, SetCookie, SetHeader};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::accounts::*;
#[cfg(feature = "server")]
use crate::server::{conn, db_err, random_token, user_err};
#[cfg(feature = "server")]
use dioxus::fullstack::{HeaderMap, StatusCode};
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
    let bad = |msg: &str| user_err(StatusCode::BAD_REQUEST, msg);
    let username = check_username(&username).map_err(bad)?;
    let display_name = display_name.trim().to_string();
    if display_name.is_empty() {
        return Err(bad("Informe seu nome."));
    }
    check_password(&password).map_err(bad)?;
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

/// A própria conta: nome, e-mail confirmado e se é administrador.
#[get("/api/conta", headers: HeaderMap)]
pub async fn my_account() -> Result<Account, ServerFnError> {
    let conn = conn().await?;
    let user = require_user(&conn, &headers).await?;
    let (user, email) = find_account(&conn, &user.username)
        .await?
        .ok_or_else(|| user_err(StatusCode::UNAUTHORIZED, "Entre na sua conta primeiro."))?;
    let is_admin = is_admin(&user);
    Ok(Account {
        user,
        email,
        is_admin,
    })
}

/// Manda um código para confirmar o e-mail novo da conta.
#[post("/api/conta/email", headers: HeaderMap)]
pub async fn request_email_code(email: String) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let user = require_user(&conn, &headers).await?;
    let email = check_email(&email).map_err(|m| user_err(StatusCode::BAD_REQUEST, m))?;
    if find_account(&conn, &email)
        .await?
        .is_some_and(|(other, _)| other.id != user.id)
    {
        return Err(user_err(
            StatusCode::CONFLICT,
            "Esse e-mail já está em outra conta.",
        ));
    }
    if recently_sent(&conn, user.id, "email").await? {
        return Err(user_err(
            StatusCode::TOO_MANY_REQUESTS,
            "Acabamos de mandar um código. Espere um minuto para pedir outro.",
        ));
    }
    let code = new_code();
    store_code(&conn, user.id, "email", &code, Some(&email), CODE_TTL).await?;
    send_email(
        &email,
        "Confirme seu e-mail no Placar de Dominó",
        &format!("Seu código é {code}. Vale por 15 minutos."),
    );
    Ok(())
}

/// Confirma o e-mail com o código recebido.
#[post("/api/conta/email/confirmar", headers: HeaderMap)]
pub async fn confirm_email(code: String) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    let user = require_user(&conn, &headers).await?;
    let email = redeem_code(&conn, user.id, &["email"], &code)
        .await?
        .ok_or_else(|| db_err("código de e-mail sem e-mail"))?;
    conn.execute(
        "UPDATE users SET email = ?1 WHERE id = ?2",
        params![email, user.id],
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            user_err(StatusCode::CONFLICT, "Esse e-mail já está em outra conta.")
        } else {
            db_err(e)
        }
    })?;
    Ok(())
}

/// Manda um código de entrada para o e-mail da conta. A resposta é a mesma
/// exista ou não a conta, para ninguém descobrir quem tem conta aqui.
#[post("/api/codigo")]
pub async fn request_login_code(who: String) -> Result<(), ServerFnError> {
    let conn = conn().await?;
    if let Some((user, Some(email))) = find_account(&conn, &who).await?
        && !recently_sent(&conn, user.id, "login").await?
    {
        let code = new_code();
        store_code(&conn, user.id, "login", &code, None, CODE_TTL).await?;
        send_email(
            &email,
            "Seu código para entrar no Placar de Dominó",
            &format!("Seu código é {code}. Vale por 15 minutos. Se não foi você, ignore."),
        );
    }
    Ok(())
}

/// Entra com o código recebido por e-mail, sem senha.
#[post("/api/codigo/entrar")]
pub async fn login_with_code(who: String, code: String) -> Result<Login, ServerFnError> {
    let conn = conn().await?;
    let wrong = || {
        user_err(
            StatusCode::BAD_REQUEST,
            "Código errado ou vencido. Confira ou peça outro.",
        )
    };
    let (user, _) = find_account(&conn, &who).await?.ok_or_else(wrong)?;
    redeem_code(&conn, user.id, &["login"], &code).await?;
    start_session(&conn, user).await
}

/// Troca a senha com o código do e-mail ou com o link do administrador.
/// Encerra as outras sessões da conta e entra.
#[post("/api/nova-senha")]
pub async fn reset_password(
    who: String,
    code: String,
    password: String,
) -> Result<Login, ServerFnError> {
    check_password(&password).map_err(|m| user_err(StatusCode::BAD_REQUEST, m))?;
    let conn = conn().await?;
    let wrong = || {
        user_err(
            StatusCode::BAD_REQUEST,
            "Código errado ou vencido. Confira ou peça outro.",
        )
    };
    let (user, _) = find_account(&conn, &who).await?.ok_or_else(wrong)?;
    redeem_code(&conn, user.id, &["login", "link"], &code).await?;
    conn.execute(
        "UPDATE users SET password_hash = ?1 WHERE id = ?2",
        params![hash_password(&password)?, user.id],
    )
    .await
    .map_err(db_err)?;
    conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user.id])
        .await
        .map_err(db_err)?;
    start_session(&conn, user).await
}

/// Administrador: gera um link de troca de senha para alguém sem e-mail.
/// Devolve o caminho do link (sem o endereço do site).
#[post("/api/admin/link", headers: HeaderMap)]
pub async fn admin_reset_link(username: String) -> Result<String, ServerFnError> {
    let conn = conn().await?;
    let admin = require_user(&conn, &headers).await?;
    if !is_admin(&admin) {
        return Err(user_err(StatusCode::FORBIDDEN, "Só o administrador pode."));
    }
    let (user, _) = find_account(&conn, &username)
        .await?
        .ok_or_else(|| user_err(StatusCode::BAD_REQUEST, "Esse usuário não existe."))?;
    let token = random_token(
        24,
        b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789",
    );
    store_code(&conn, user.id, "link", &token, None, LINK_TTL).await?;
    Ok(format!("/nova-senha/{}/{token}", user.username))
}
