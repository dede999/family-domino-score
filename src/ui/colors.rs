use crate::api::{self, error_message};
use crate::model::{RoomView, pair_key, resolve_side_colors, sides_of};
use crate::palette::{self, PALETTE};
use dioxus::prelude::*;

/// Bolinha de cor. Com `onclick`, vira botão.
#[component]
pub fn Swatch(
    color: u8,
    #[props(default)] selected: bool,
    #[props(default)] disabled: bool,
    onclick: Option<EventHandler<()>>,
) -> Element {
    let name = PALETTE[(color as usize).min(PALETTE.len() - 1)].name;
    match onclick {
        Some(onclick) => rsx! {
            button {
                r#type: "button",
                class: if selected { "swatch on" } else { "swatch" },
                style: palette::style(color),
                title: name,
                "aria-label": name,
                "aria-pressed": selected,
                disabled,
                onclick: move |_| onclick.call(()),
            }
        },
        None => rsx! {
            span {
                class: "swatch",
                style: palette::style(color),
                title: name,
                role: "img",
                "aria-label": name,
            }
        },
    }
}

/// Fileira com todas as cores da paleta. As de `taken` ficam desativadas.
#[component]
pub fn ColorPicker(
    value: u8,
    #[props(default)] taken: Vec<u8>,
    onchange: EventHandler<u8>,
) -> Element {
    rsx! {
        div { class: "color-picker",
            for c in 0..PALETTE.len() as u8 {
                Swatch {
                    key: "{c}",
                    color: c,
                    selected: c == value,
                    disabled: c != value && taken.contains(&c),
                    onclick: move |_| onchange.call(c),
                }
            }
        }
    }
}

/// Cores das duplas da roda atual. Cada dupla escolhe uma principal e uma reserva.
#[component]
pub fn PairColorsCard(view: RoomView, on_change: EventHandler<()>) -> Element {
    let mut open = use_signal(|| None::<usize>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let seating: Vec<i64> = view.players.iter().map(|p| p.id).collect();
    let sides = sides_of(view.mode, &seating);
    let effective = resolve_side_colors(view.mode, &seating, &view.players, &view.pairs);

    let save = use_callback(
        move |(code, ids, primary, secondary): (String, (i64, i64), u8, u8)| {
            busy.set(true);
            error.set(None);
            spawn(async move {
                let result = api::set_pair_colors(code, ids, primary, secondary).await;
                busy.set(false);
                if let Err(e) = result {
                    error.set(Some(error_message(&e)));
                }
                on_change.call(());
            });
        },
    );

    rsx! {
        section { class: "card",
            h2 { "Cores das duplas" }
            p { class: "muted small",
                "Se as duas duplas escolherem a mesma principal, quem escolheu por último usa a reserva."
            }
            for (side, ids) in sides.iter().enumerate() {
                {
                    let key = pair_key(ids[0], ids[1]);
                    let stored = view.pairs.iter().find(|p| p.players == key);
                    let primary = stored.map_or(effective[side], |p| p.primary);
                    let secondary = stored.map_or_else(|| palette::first_free(&effective), |p| p.secondary);
                    let code = view.code.clone();
                    let code2 = view.code.clone();
                    rsx! {
                        div { key: "{key.0}-{key.1}", class: "pair-colors",
                            div { class: "row spread",
                                span { class: "pair-name",
                                    Swatch { color: effective[side] }
                                    "{view.pair_name(ids)}"
                                }
                                if open() != Some(side) {
                                    button { class: "link", onclick: move |_| open.set(Some(side)), "Mudar" }
                                } else {
                                    button { class: "link", onclick: move |_| open.set(None), "Pronto" }
                                }
                            }
                            if open() == Some(side) {
                                fieldset { class: "picker-group", disabled: busy(),
                                    legend { "Principal" }
                                    ColorPicker {
                                        value: primary,
                                        onchange: move |c: u8| {
                                            // A reserva não pode ser igual à principal.
                                            let reserve = if c == secondary { primary } else { secondary };
                                            save.call((code.clone(), key, c, reserve));
                                        },
                                    }
                                }
                                fieldset { class: "picker-group", disabled: busy(),
                                    legend { "Reserva" }
                                    ColorPicker {
                                        value: secondary,
                                        taken: vec![primary],
                                        onchange: move |c: u8| save.call((code2.clone(), key, primary, c)),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(err) = error() {
                p { class: "error", "{err}" }
            }
        }
    }
}
