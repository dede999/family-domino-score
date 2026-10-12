mod home;
mod new_room;
mod room;

pub use home::Home;
pub use new_room::NewRoom;
pub use room::Room;

use crate::model::UserInfo;
use dioxus::prelude::*;

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

/// A cruz do placar: uma bolinha na ponta de cada braço, preenchidas na ordem
/// cima, direita, baixo, esquerda.
#[component]
pub fn Cross(score: u32, winner: bool) -> Element {
    let ends = [(50, 14), (86, 50), (50, 86), (14, 50)];
    rsx! {
        svg {
            class: if winner { "cross winner" } else { "cross" },
            view_box: "0 0 100 100",
            role: "img",
            "aria-label": "{score} de 4",
            line { x1: 50, y1: 14, x2: 50, y2: 86 }
            line { x1: 14, y1: 50, x2: 86, y2: 50 }
            for (i, (cx, cy)) in ends.into_iter().enumerate() {
                circle {
                    key: "{i}",
                    class: if (i as u32) < score { "ball on" } else { "ball" },
                    cx,
                    cy,
                    r: 10,
                }
            }
        }
    }
}
