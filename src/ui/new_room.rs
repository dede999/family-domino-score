use super::{current_user, use_me_resource};
use crate::Route;
use crate::api::{self, error_message};
use crate::model::{Mode, PlayerInput};
use dioxus::prelude::*;

/// Linha editável de jogador: nome e, opcionalmente, a conta vinculada.
#[derive(Clone, Default, PartialEq)]
pub struct PlayerDraft {
    pub id: Option<i64>,
    pub name: String,
    pub username: String,
}

impl PlayerDraft {
    pub fn to_input(&self) -> PlayerInput {
        let username = self.username.trim().trim_start_matches('@').to_string();
        PlayerInput {
            id: self.id,
            name: self.name.trim().to_string(),
            username: (!username.is_empty()).then_some(username),
        }
    }
}

#[component]
pub fn NewRoom() -> Element {
    let me = use_me_resource();
    let nav = use_navigator();
    let mut name = use_signal(String::new);
    let mut mode = use_signal(|| Mode::Duplas);
    let mut players = use_signal(|| vec![PlayerDraft::default(); 4]);
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    // Preenche o primeiro lugar com o dono da sala.
    let user = current_user(me);
    use_effect(move || {
        if let Some(u) = current_user(me) {
            let mut list = players.write();
            if list[0].username.is_empty() && list[0].name.is_empty() {
                list[0] = PlayerDraft {
                    id: None,
                    name: u.display_name.clone(),
                    username: u.username.clone(),
                };
            }
        }
    });

    if me.read().is_some() && user.is_none() {
        return rsx! {
            main { class: "page",
                p { "Entre na sua conta para criar uma sala." }
                Link { to: Route::Home {}, "Voltar" }
            }
        };
    }

    let submit = move |e: FormEvent| {
        e.prevent_default();
        busy.set(true);
        error.set(None);
        let list: Vec<PlayerInput> = players().iter().map(PlayerDraft::to_input).collect();
        spawn(async move {
            match api::create_room(name(), mode(), list).await {
                Ok(code) => {
                    nav.replace(Route::Room { code });
                }
                Err(e) => {
                    busy.set(false);
                    error.set(Some(error_message(&e)));
                }
            }
        });
    };

    rsx! {
        main { class: "page",
            header { class: "top",
                Link { to: Route::Home {}, "← Voltar" }
                h1 { "Nova sala" }
            }
            form { class: "card stack", onsubmit: submit,
                label {
                    "Nome da sala"
                    input {
                        placeholder: "Ex.: Dominó lá de casa",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                fieldset { class: "segmented",
                    legend { "Modo de jogo" }
                    button {
                        r#type: "button",
                        class: if mode() == Mode::Duplas { "on" },
                        onclick: move |_| {
                            mode.set(Mode::Duplas);
                            players.write().resize(4, PlayerDraft::default());
                        },
                        "Duplas"
                    }
                    button {
                        r#type: "button",
                        class: if mode() == Mode::Individual { "on" },
                        onclick: move |_| mode.set(Mode::Individual),
                        "Individual"
                    }
                }
                PlayersEditor { mode: mode(), players }
                if let Some(err) = error() {
                    p { class: "error", "{err}" }
                }
                button { r#type: "submit", disabled: busy(), "Criar sala" }
            }
        }
    }
}

/// Editor da roda de jogadores, usado na criação e na edição da sala.
/// Nas duplas, a ordem da roda é A1, B1, A2, B2 (parceiros frente a frente).
#[component]
pub fn PlayersEditor(mode: Mode, players: Signal<Vec<PlayerDraft>>) -> Element {
    let count = players.read().len();
    rsx! {
        div { class: "players-editor",
            p { class: "muted",
                if mode == Mode::Duplas {
                    "Os parceiros sentam frente a frente. A ordem abaixo é a ordem da roda, que define quem sai depois de cada mão."
                } else {
                    "De 2 a 4 jogadores, na ordem em que estão sentados. Essa ordem define quem sai depois de cada mão."
                }
            }
            p { class: "muted",
                "Deixe a conta em branco para um jogador avulso. Dá para vincular a conta depois."
            }
            for seat in 0..count {
                div { key: "{seat}", class: "player-row",
                    span { class: "seat",
                        {seat_label(mode, seat)}
                    }
                    input {
                        placeholder: "Nome",
                        value: "{players.read()[seat].name}",
                        oninput: move |e| players.write()[seat].name = e.value(),
                    }
                    input {
                        class: "username",
                        placeholder: "@conta (opcional)",
                        autocapitalize: "none",
                        value: "{players.read()[seat].username}",
                        oninput: move |e| players.write()[seat].username = e.value(),
                    }
                    if seat > 0 {
                        button {
                            r#type: "button",
                            class: "icon",
                            title: "Subir na roda",
                            onclick: move |_| players.write().swap(seat - 1, seat),
                            "↑"
                        }
                    }
                    if mode == Mode::Individual && count > 2 {
                        button {
                            r#type: "button",
                            class: "icon",
                            title: "Remover",
                            onclick: move |_| {
                                players.write().remove(seat);
                            },
                            "×"
                        }
                    }
                }
            }
            if mode == Mode::Individual && count < 4 {
                button {
                    r#type: "button",
                    class: "link",
                    onclick: move |_| players.write().push(PlayerDraft::default()),
                    "+ Adicionar jogador"
                }
            }
        }
    }
}

pub fn seat_label(mode: Mode, seat: usize) -> String {
    match mode {
        Mode::Duplas => format!(
            "Dupla {} ",
            if seat % 2 == 0 { "A" } else { "B" }
        ),
        Mode::Individual => format!("{}º", seat + 1),
    }
}
