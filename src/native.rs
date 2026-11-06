//! Só no app Android (feature `mobile`). A interface é a mesma do site, mas
//! não vem do servidor: as chamadas vão para o endereço publicado, e a
//! sessão precisa ser guardada pelo próprio app.
//!
//! O cookie de sessão do app só vive na memória e some quando o app fecha.
//! Por isso, depois de entrar, o app pede o token (`api::session_token`),
//! guarda no `localStorage` da WebView e o manda como cookie em todas as
//! chamadas. Ao abrir de novo, usa o guardado.

use crate::api;
use crate::views::UserInfo;
use dioxus::fullstack::{HeaderMap, HeaderValue, clear_request_headers, set_request_headers};
use dioxus::prelude::*;

/// Servidor publicado. `PLACAR_SERVER` na compilação troca o endereço (por
/// exemplo, para testar com o servidor de casa).
const SERVER_URL: &str = match option_env!("PLACAR_SERVER") {
    Some(url) => url,
    None => "https://placar-276818208775.southamerica-east1.run.app",
};

/// Mesmo nome de `accounts::SESSION_COOKIE`, que só existe no servidor.
const SESSION_COOKIE: &str = "placar_session";
const STORAGE_KEY: &str = "sessao";

/// Antes de abrir a interface.
pub fn setup() {
    dioxus::fullstack::set_server_url(SERVER_URL);
}

/// Quem está logado, guardando ou esquecendo a sessão do app. Substitui
/// `api::me` no `App`.
pub async fn me() -> Result<Option<UserInfo>, ServerFnError> {
    // Primeiro sem o cookie guardado: logo depois de entrar, quem vale é o
    // cookie novo, que o cliente HTTP já tem na memória.
    clear_request_headers();
    if let Some(user) = api::me().await? {
        if let Some(token) = api::session_token().await? {
            store(Some(&token)).await;
            use_token(&token);
        }
        return Ok(Some(user));
    }
    // App recém-aberto: tenta a sessão guardada.
    let Some(token) = stored().await else {
        return Ok(None);
    };
    use_token(&token);
    let user = api::me().await?;
    if user.is_none() {
        // Saiu, trocou a senha ou excluiu a conta.
        clear_request_headers();
        store(None).await;
    }
    Ok(user)
}

fn use_token(token: &str) {
    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&format!("{SESSION_COOKIE}={token}")) {
        headers.insert(dioxus::fullstack::http::header::COOKIE, value);
    }
    set_request_headers(headers);
}

async fn stored() -> Option<String> {
    document::eval(&format!(
        "try {{ return localStorage.getItem('{STORAGE_KEY}'); }} catch (e) {{ return null; }}"
    ))
    .await
    .ok()
    .and_then(|v| v.as_str().map(String::from))
}

async fn store(token: Option<&str>) {
    // O token tem só letras e números (`accounts::create_session`); vai
    // direto no JavaScript.
    let js = match token {
        Some(t) if !t.chars().all(|c| c.is_ascii_alphanumeric()) => return,
        Some(t) => format!("localStorage.setItem('{STORAGE_KEY}', '{t}')"),
        None => format!("localStorage.removeItem('{STORAGE_KEY}')"),
    };
    let _ = document::eval(&format!("try {{ {js} }} catch (e) {{}}")).await;
}

/// Endereço do site, para montar links que saem do app.
pub fn site_origin() -> &'static str {
    SERVER_URL
}
