//! Barra do topo de todas as telas: à esquerda o que é da tela (título,
//! usuário ou "← Início"); à direita, sempre no mesmo lugar, o tema e a
//! ajuda ("?") da tela.

use super::help::{Help, HelpButton};
use super::theme::ThemeButton;
use dioxus::prelude::*;

#[component]
pub fn TopBar(help: Help, children: Element) -> Element {
    rsx! {
        header { class: "top",
            div { class: "top-main", {children} }
            div { class: "top-tools",
                ThemeButton {}
                HelpButton { help }
            }
        }
    }
}
