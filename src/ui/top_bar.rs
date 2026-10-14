//! Barra do topo de todas as telas: à esquerda o que é da tela (título,
//! usuário ou "← Início"); à direita, sempre no mesmo lugar, as ferramentas.

use super::theme::ThemeButton;
use dioxus::prelude::*;

#[component]
pub fn TopBar(children: Element) -> Element {
    rsx! {
        header { class: "top",
            div { class: "top-main", {children} }
            div { class: "top-tools", ThemeButton {} }
        }
    }
}
