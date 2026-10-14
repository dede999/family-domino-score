mod account;
mod colors;
mod drawer;
mod history;
mod home;
mod new_room;
mod room;
mod theme;
mod top_bar;

pub use account::{EnterWithCode, ForgotPassword, LinkReset, MyAccount};
pub use home::Home;
pub use new_room::NewRoom;
pub use room::Room;
pub use theme::THEME_SCRIPT;

use crate::model::Outcome;
use crate::palette;
use crate::views::UserInfo;
use dioxus::prelude::*;
pub use top_bar::TopBar;

pub type MeResource = Resource<Result<Option<UserInfo>, ServerFnError>>;

/// Recurso do usuário logado, criado no `App`.
pub fn use_me_resource() -> MeResource {
    use_context()
}

/// Usuário logado, ou `None` enquanto carrega ou se não estiver logado.
pub fn current_user(me: MeResource) -> Option<UserInfo> {
    match &*me.read() {
        Some(Ok(user)) => user.clone(),
        _ => None,
    }
}

pub async fn sleep_ms(ms: u32) {
    #[cfg(feature = "web")]
    gloo_timers::future::TimeoutFuture::new(ms).await;
    #[cfg(not(feature = "web"))]
    let _ = ms;
}

/// Espera a página ficar escondida (`hidden = true`: outra aba, app em
/// segundo plano, tela apagada) ou visível de novo. Volta na hora se a
/// página já estiver assim.
pub async fn page_hidden(hidden: bool) {
    let mut wait = document::eval(&format!(
        r#"
        const want = {hidden};
        if (document.hidden !== want) {{
            await new Promise((done) => {{
                const check = () => {{
                    if (document.hidden === want) {{
                        document.removeEventListener("visibilitychange", check);
                        done();
                    }}
                }};
                document.addEventListener("visibilitychange", check);
            }});
        }}
        dioxus.send(true);
        "#
    ));
    let _ = wait.recv::<bool>().await;
}

/// Como a cruz aparece no fim do boi.
#[derive(Clone, Copy, PartialEq)]
pub enum CrossEnd {
    /// Boi em andamento.
    Playing,
    /// Bolinhas douradas.
    Winner,
    /// Bolinhas acinzentadas.
    Loser,
    /// Perdeu sem marcar nada: braços em vermelho.
    Lambreta,
}

/// A cruz do placar: uma bolinha na ponta de cada braço, preenchidas no
/// sentido anti-horário (cima, esquerda, baixo, direita). Batida é bolinha
/// cheia na cor do lado; fechado é casca na cor com miolo claro.
#[component]
pub fn Cross(wins: Vec<Outcome>, color: u8, end: CrossEnd) -> Element {
    let ends = [(50, 14), (14, 50), (50, 86), (86, 50)];
    let class = match end {
        CrossEnd::Playing => "cross",
        CrossEnd::Winner => "cross winner",
        CrossEnd::Loser => "cross loser",
        CrossEnd::Lambreta => "cross loser lambreta",
    };
    let score = wins.len().min(4);
    rsx! {
        svg {
            class,
            style: palette::style(color),
            view_box: "0 0 100 100",
            role: "img",
            "aria-label": "{score} de 4",
            line { x1: 50, y1: 14, x2: 50, y2: 86 }
            line { x1: 14, y1: 50, x2: 86, y2: 50 }
            for (i, (cx, cy)) in ends.into_iter().enumerate() {
                circle {
                    key: "{i}",
                    class: match wins.get(i) {
                        Some(Outcome::Batida) => "ball batida",
                        Some(_) => "ball fechado",
                        None => "ball",
                    },
                    cx,
                    cy,
                    r: 10,
                }
            }
        }
    }
}
