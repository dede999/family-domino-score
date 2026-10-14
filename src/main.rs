use dioxus::prelude::*;

#[cfg(feature = "server")]
mod accounts;
mod api;
mod model;
mod palette;
mod period;
// Parte das regras e dos tipos só é usada no servidor.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
mod room_rules;
#[cfg_attr(not(feature = "server"), allow(dead_code))]
mod rules;
#[cfg(feature = "server")]
mod server;
mod ui;
mod views;

use ui::{EnterWithCode, ForgotPassword, Home, LinkReset, MyAccount, NewRoom, Room};

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/nova-sala")]
    NewRoom {},
    #[route("/sala/:code")]
    Room { code: String },
    #[route("/conta")]
    MyAccount {},
    #[route("/entrar-com-codigo")]
    EnterWithCode {},
    #[route("/esqueci-a-senha")]
    ForgotPassword {},
    #[route("/nova-senha/:username/:token")]
    LinkReset { username: String, token: String },
}

const CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Usuário logado, disponível para todas as telas. `restart()` depois de entrar ou sair.
    let me = use_resource(api::me);
    use_context_provider(|| me);

    // Efeitos só rodam no navegador, depois da hidratação. Antes disso a página
    // veio pronta do servidor, mas os cliques ainda não fazem nada. O teste da
    // interface espera por esta marca.
    use_effect(|| {
        document::eval("document.documentElement.dataset.pronto = '1'");
    });

    rsx! {
        document::Title { "Placar de Dominó" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1" }
        document::Stylesheet { href: CSS }
        // Aplica o tema antes de a página aparecer (ver `ui::theme`).
        document::Script { "{ui::THEME_SCRIPT}" }
        Router::<Route> {}
    }
}
