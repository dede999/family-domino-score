//! Telas da conta: entrar com código, esqueci a senha, minha conta e troca
//! de senha pelo link do administrador.

use super::{current_user, use_me_resource};
use crate::Route;
use crate::api::{self, error_message};
use dioxus::prelude::*;

#[component]
pub fn EnterWithCode() -> Element {
    rsx! { CodeLogin { reset: false } }
}

#[component]
pub fn ForgotPassword() -> Element {
    rsx! { CodeLogin { reset: true } }
}

/// Entrar com código (sem senha) ou trocar a senha esquecida. Primeiro pede
/// o usuário ou e-mail e manda o código; depois confere o código.
#[component]
fn CodeLogin(reset: bool) -> Element {
    let mut me = use_me_resource();
    let nav = use_navigator();
    let mut who = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut sent = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let send = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            match api::request_login_code(who()).await {
                Ok(()) => sent.set(true),
                Err(e) => error.set(Some(error_message(&e))),
            }
            busy.set(false);
        });
    };
    let enter = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = if reset {
                api::reset_password(who(), code(), password()).await
            } else {
                api::login_with_code(who(), code()).await
            };
            busy.set(false);
            match result {
                Ok(_) => {
                    me.restart();
                    nav.push(Route::Home {});
                }
                Err(e) => error.set(Some(error_message(&e))),
            }
        });
    };

    rsx! {
        main { class: "page",
            header { class: "top",
                Link { to: Route::Home {}, "← Início" }
            }
            section { class: "card",
                h2 { if reset { "Esqueci a senha" } else { "Entrar com código" } }
                if !sent() {
                    p { class: "muted",
                        "Mandamos um código para o e-mail da sua conta. Só funciona se você já confirmou um e-mail em \"Minha conta\"."
                    }
                    form { class: "stack", onsubmit: send,
                        label {
                            "Usuário ou e-mail"
                            input {
                                value: "{who}",
                                autocomplete: "username",
                                autocapitalize: "none",
                                oninput: move |e| who.set(e.value()),
                            }
                        }
                        if let Some(err) = error() {
                            p { class: "error", "{err}" }
                        }
                        button { r#type: "submit", disabled: busy(), "Mandar código" }
                    }
                } else {
                    p {
                        "Se essa conta tiver e-mail confirmado, o código chegou lá. Confira também a pasta de spam. Ele vale por 15 minutos."
                    }
                    form { class: "stack", onsubmit: enter,
                        label {
                            "Código"
                            input {
                                value: "{code}",
                                inputmode: "numeric",
                                autocomplete: "one-time-code",
                                oninput: move |e| code.set(e.value()),
                            }
                        }
                        if reset {
                            label {
                                "Senha nova"
                                input {
                                    r#type: "password",
                                    value: "{password}",
                                    autocomplete: "new-password",
                                    oninput: move |e| password.set(e.value()),
                                }
                            }
                        }
                        if let Some(err) = error() {
                            p { class: "error", "{err}" }
                        }
                        button { r#type: "submit", disabled: busy(),
                            if reset { "Trocar a senha e entrar" } else { "Entrar" }
                        }
                    }
                    button {
                        class: "link",
                        onclick: move |_| {
                            sent.set(false);
                            error.set(None);
                        },
                        "Pedir outro código"
                    }
                }
                p { class: "muted small",
                    "Sem e-mail na conta? Peça a quem cuida do app um link para trocar a senha."
                }
            }
        }
    }
}

/// Troca de senha pelo link que o administrador gerou.
#[component]
pub fn LinkReset(username: String, token: String) -> Element {
    let mut me = use_me_resource();
    let nav = use_navigator();
    let mut password = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let who = username.clone();

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        let (who, token) = (who.clone(), token.clone());
        spawn(async move {
            let result = api::reset_password(who, token, password()).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    me.restart();
                    nav.push(Route::Home {});
                }
                Err(e) => error.set(Some(error_message(&e))),
            }
        });
    };

    rsx! {
        main { class: "page",
            header { class: "top",
                Link { to: Route::Home {}, "← Início" }
            }
            section { class: "card",
                h2 { "Senha nova" }
                p { class: "muted", "Conta: {username}" }
                form { class: "stack", onsubmit: submit,
                    label {
                        "Senha nova"
                        input {
                            r#type: "password",
                            value: "{password}",
                            autocomplete: "new-password",
                            oninput: move |e| password.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        p { class: "error", "{err}" }
                    }
                    button { r#type: "submit", disabled: busy(), "Trocar a senha e entrar" }
                }
            }
        }
    }
}

/// Minha conta: e-mail (para entrar com código) e, para o administrador,
/// links de troca de senha.
#[component]
pub fn MyAccount() -> Element {
    let me = use_me_resource();
    let mut account = use_resource(api::my_account);
    let body = match &*account.read() {
        _ if me.read().is_some() && current_user(me).is_none() => rsx! {
            p { "Entre na sua conta para ver esta página." }
        },
        None => rsx! { p { class: "muted", "Carregando…" } },
        Some(Err(e)) => rsx! { p { class: "error", "{error_message(e)}" } },
        Some(Ok(acc)) => rsx! {
            section { class: "card",
                h2 { "Minha conta" }
                dl { class: "stats",
                    dt { "Nome" }
                    dd { "{acc.user.display_name}" }
                    dt { "Usuário" }
                    dd { "{acc.user.username}" }
                    dt { "E-mail" }
                    dd {
                        match &acc.email {
                            Some(email) => rsx! { "{email}" },
                            None => rsx! { span { class: "muted", "nenhum" } },
                        }
                    }
                }
            }
            EmailCard { has_email: acc.email.is_some(), on_done: move |_| account.restart() }
            if acc.is_admin {
                AdminLinkCard {}
            }
        },
    };
    rsx! {
        main { class: "page",
            header { class: "top",
                Link { to: Route::Home {}, "← Início" }
            }
            {body}
        }
    }
}

#[component]
fn EmailCard(has_email: bool, on_done: EventHandler<()>) -> Element {
    let mut email = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut sent = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let send = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            match api::request_email_code(email()).await {
                Ok(()) => sent.set(true),
                Err(e) => error.set(Some(error_message(&e))),
            }
            busy.set(false);
        });
    };
    let confirm = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            match api::confirm_email(code()).await {
                Ok(()) => {
                    sent.set(false);
                    email.set(String::new());
                    code.set(String::new());
                    on_done.call(());
                }
                Err(e) => error.set(Some(error_message(&e))),
            }
            busy.set(false);
        });
    };

    rsx! {
        section { class: "card",
            h2 { if has_email { "Trocar e-mail" } else { "Adicionar e-mail" } }
            p { class: "muted",
                "Com um e-mail confirmado, você pode entrar com um código e trocar a senha se esquecer."
            }
            if !sent() {
                form { class: "stack", onsubmit: send,
                    label {
                        "E-mail"
                        input {
                            r#type: "email",
                            value: "{email}",
                            autocomplete: "email",
                            autocapitalize: "none",
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        p { class: "error", "{err}" }
                    }
                    button { r#type: "submit", disabled: busy(), "Mandar código" }
                }
            } else {
                form { class: "stack", onsubmit: confirm,
                    p { "Mandamos um código para {email}. Ele vale por 15 minutos." }
                    label {
                        "Código"
                        input {
                            value: "{code}",
                            inputmode: "numeric",
                            autocomplete: "one-time-code",
                            oninput: move |e| code.set(e.value()),
                        }
                    }
                    if let Some(err) = error() {
                        p { class: "error", "{err}" }
                    }
                    button { r#type: "submit", disabled: busy(), "Confirmar e-mail" }
                }
                button {
                    class: "link",
                    onclick: move |_| {
                        sent.set(false);
                        error.set(None);
                    },
                    "Usar outro e-mail"
                }
            }
        }
    }
}

/// Administrador: gera o link de troca de senha e abre o WhatsApp com a
/// mensagem pronta, para mandar do próprio celular.
#[component]
fn AdminLinkCard() -> Element {
    let mut username = use_signal(String::new);
    let mut link = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        link.set(None);
        spawn(async move {
            match api::admin_reset_link(username()).await {
                Ok(path) => {
                    let origin = document::eval("return location.origin")
                        .await
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default();
                    link.set(Some(format!("{origin}{path}")));
                }
                Err(e) => error.set(Some(error_message(&e))),
            }
            busy.set(false);
        });
    };

    rsx! {
        section { class: "card",
            h2 { "Trocar a senha de alguém" }
            p { class: "muted",
                "Para quem não tem e-mail na conta. O link vale por 1 hora e só uma vez."
            }
            form { class: "row", onsubmit: submit,
                input {
                    placeholder: "Usuário",
                    value: "{username}",
                    autocapitalize: "none",
                    oninput: move |e| username.set(e.value()),
                }
                button { r#type: "submit", disabled: busy(), "Gerar link" }
            }
            if let Some(err) = error() {
                p { class: "error", "{err}" }
            }
            if let Some(url) = link() {
                p { class: "link-out", "{url}" }
                a {
                    class: "button",
                    href: whatsapp_url(&format!(
                        "Oi! Para trocar sua senha do Placar de Dominó, abra este link (vale por 1 hora): {url}"
                    )),
                    target: "_blank",
                    rel: "noopener",
                    "Mandar pelo WhatsApp"
                }
            }
        }
    }
}

/// Link do WhatsApp com a mensagem pronta; a pessoa escolhe o contato.
fn whatsapp_url(text: &str) -> String {
    let encoded: String = text
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("https://wa.me/?text={encoded}")
}
