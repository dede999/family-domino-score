//! Gaveta que sobe de baixo, com título e botão "Fechar". Fecha também
//! tocando fora dela.

use dioxus::prelude::*;

#[component]
pub fn Drawer(
    title: String,
    /// Linha discreta abaixo do título (ex.: a data do boi).
    subtitle: Option<String>,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "drawer-backdrop", onclick: move |_| on_close.call(()),
            div {
                class: "drawer",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{title}",
                onclick: move |e| e.stop_propagation(),
                div { class: "drawer-head",
                    div {
                        h2 { "{title}" }
                        if let Some(s) = subtitle {
                            p { class: "muted", "{s}" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "secondary",
                        onclick: move |_| on_close.call(()),
                        "Fechar"
                    }
                }
                {children}
            }
        }
    }
}
