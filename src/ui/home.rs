use super::{Help, HelpInvite, TopBar, current_user, use_me_resource};
use crate::Route;
use crate::api::{self, error_message};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let me = use_me_resource();
    let user = current_user(me);
    let loading = me.read().is_none();

    rsx! {
        main { class: "page",
            TopBar { help: Help::Home,
                h1 { "Placar de Dominó" }
            }
            // Fora da barra do topo: com o tema e o "?" ao lado, não sobra
            // largura no celular para o nome e os links.
            if let Some(user) = &user {
                div { class: "who",
                    span { "Olá, {user.display_name}" }
                    Link { to: Route::MyAccount {}, "Minha conta" }
                    LogoutButton {}
                }
            }
            HelpInvite {}
            JoinRoom {}
            if loading {
                p { class: "muted", "Carregando…" }
            } else if user.is_some() {
                MyArea {}
            } else {
                AuthForm {}
            }
        }
    }
}

#[component]
fn JoinRoom() -> Element {
    let mut code = use_signal(String::new);
    let nav = use_navigator();
    rsx! {
        section { class: "card",
            h2 { "Acompanhar uma sala" }
            p { class: "muted", "Quem tem o código da sala vê o placar ao vivo, sem precisar de conta." }
            form {
                class: "row",
                onsubmit: move |e: FormEvent| {
                    e.prevent_default();
                    let c = code().trim().to_uppercase();
                    if !c.is_empty() {
                        nav.push(Route::Room { code: c });
                    }
                },
                input {
                    placeholder: "Código da sala",
                    value: "{code}",
                    autocapitalize: "characters",
                    oninput: move |e| code.set(e.value()),
                }
                button { r#type: "submit", "Abrir" }
            }
        }
    }
}

#[component]
fn AuthForm() -> Element {
    let mut me = use_me_resource();
    let mut creating = use_signal(|| false);
    let mut username = use_signal(String::new);
    let mut display_name = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = if creating() {
                api::register(username(), display_name(), password()).await
            } else {
                api::login(username(), password()).await
            };
            busy.set(false);
            match result {
                Ok(_) => me.restart(),
                Err(e) => error.set(Some(error_message(&e))),
            }
        });
    };

    rsx! {
        section { class: "card",
            h2 { if creating() { "Criar conta" } else { "Entrar" } }
            p { class: "muted", "Com uma conta você cria salas e marca o placar." }
            form { class: "stack", onsubmit: submit,
                label {
                    "Usuário"
                    input {
                        value: "{username}",
                        autocomplete: "username",
                        autocapitalize: "none",
                        oninput: move |e| username.set(e.value()),
                    }
                }
                if creating() {
                    label {
                        "Seu nome (aparece no placar)"
                        input {
                            value: "{display_name}",
                            oninput: move |e| display_name.set(e.value()),
                        }
                    }
                }
                label {
                    "Senha"
                    input {
                        r#type: "password",
                        value: "{password}",
                        autocomplete: if creating() { "new-password" } else { "current-password" },
                        oninput: move |e| password.set(e.value()),
                    }
                }
                if let Some(err) = error() {
                    p { class: "error", "{err}" }
                }
                button { r#type: "submit", disabled: busy(),
                    if creating() { "Criar conta" } else { "Entrar" }
                }
            }
            button {
                class: "link",
                onclick: move |_| {
                    creating.toggle();
                    error.set(None);
                },
                if creating() { "Já tenho conta" } else { "Criar uma conta" }
            }
            if !creating() {
                p { class: "small",
                    Link { to: Route::ForgotPassword {}, "Esqueci a senha" }
                    " · "
                    Link { to: Route::EnterWithCode {}, "Entrar com código" }
                }
            }
        }
    }
}

#[component]
fn LogoutButton() -> Element {
    let mut me = use_me_resource();
    rsx! {
        button {
            class: "link",
            onclick: move |_| async move {
                let _ = api::logout().await;
                me.restart();
            },
            "Sair"
        }
    }
}

#[component]
fn MyArea() -> Element {
    let rooms = use_resource(api::my_rooms);
    let stats = use_resource(api::my_stats);

    rsx! {
        section { class: "card",
            div { class: "row spread",
                h2 { "Minhas salas" }
                Link { class: "button", to: Route::NewRoom {}, "Nova sala" }
            }
            match &*rooms.read() {
                None => rsx! { p { class: "muted", "Carregando…" } },
                Some(Err(e)) => rsx! { p { class: "error", "{error_message(e)}" } },
                Some(Ok(list)) if list.is_empty() => rsx! {
                    p { class: "muted", "Você ainda não tem salas." }
                },
                Some(Ok(list)) => rsx! {
                    ul { class: "rooms",
                        for room in list.iter() {
                            li { key: "{room.code}",
                                Link { to: Route::Room { code: room.code.clone() },
                                    strong { "{room.name}" }
                                    span { class: "muted",
                                        " · {room.code} · "
                                        {match room.mode {
                                            crate::model::Mode::Individual => "individual",
                                            crate::model::Mode::Duplas => "duplas",
                                        }}
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
        if let Some(Ok(s)) = &*stats.read() {
            if s.bois_jogados > 0 {
                section { class: "card",
                    h2 { "Minhas estatísticas" }
                    p { class: "muted", "Somando todas as salas em que você joga com a sua conta." }
                    dl { class: "stats",
                        dt { "Bois ganhos" }
                        dd { "{s.bois_ganhos} de {s.bois_jogados}" }
                        dt { "Mãos ganhas" }
                        dd { "{s.maos_ganhas} de {s.maos_jogadas} ({s.batidas} batendo, {s.fechados} no fechado)" }
                        dt { "Lambretas" }
                        dd { "{s.lambretas_aplicadas} aplicadas, {s.lambretas_sofridas} sofridas" }
                        dt { "Lambretas reversas" }
                        dd { "{s.reversas_aplicadas} aplicadas, {s.reversas_sofridas} sofridas" }
                    }
                }
            }
        }
    }
}
