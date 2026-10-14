//! Ajuda de cada tela: o botão "?" no topo abre uma gaveta que explica só a
//! tela atual. Textos curtos, com as mesmas palavras dos botões.

use super::drawer::Drawer;
use dioxus::prelude::*;

/// Qual tela pede ajuda.
#[derive(Clone, Copy, PartialEq)]
pub enum Help {
    Home,
    NewRoom,
    Room,
    Account,
    CodeLogin,
    NewPassword,
}

impl Help {
    fn title(self) -> &'static str {
        match self {
            Help::Home => "Como usar o placar",
            Help::NewRoom => "Como criar uma sala",
            Help::Room => "Como usar a sala",
            Help::Account => "Minha conta",
            Help::CodeLogin => "Entrar com código",
            Help::NewPassword => "Senha nova",
        }
    }
}

#[component]
pub fn HelpButton(help: Help) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        button {
            r#type: "button",
            class: "tool help",
            title: "Ajuda desta tela",
            "aria-label": "Ajuda desta tela",
            onclick: move |_| open.set(true),
            "?"
        }
        if open() {
            Drawer { title: help.title(), on_close: move |_| open.set(false),
                div { class: "help", {body(help)} }
            }
        }
    }
}

fn body(help: Help) -> Element {
    match help {
        Help::Home => rsx! {
            h3 { "Acompanhar uma sala" }
            p {
                "Digite o código que alguém da sala te passou e toque em "
                b { "Abrir" }
                ". Você vê o placar ao vivo, sem precisar de conta."
            }
            h3 { "Conta" }
            p {
                "Com uma conta você cria salas e marca o placar. Em "
                b { "Minhas salas" }
                ", toque numa sala para abrir, ou em "
                b { "Nova sala" }
                " para criar outra."
            }
            h3 { "Esqueceu a senha?" }
            p {
                "Use "
                b { "Esqueci a senha" }
                ", que manda um código para o seu e-mail (precisa ter confirmado o e-mail em "
                b { "Minha conta" }
                "). Sem e-mail, peça um link a quem cuida do app."
            }
            {theme_tip()}
        },
        Help::NewRoom => rsx! {
            h3 { "Modo de jogo" }
            p {
                b { "Duplas" }
                ": quatro jogadores, os parceiros sentados frente a frente. "
                b { "Individual" }
                ": de 2 a 4, cada um por si."
            }
            h3 { "Jogadores" }
            p {
                "Ponha os nomes na ordem em que estão sentados, girando pela mesa. Essa ordem define quem sai depois de cada mão. A seta "
                b { "↑" }
                " sobe o jogador na roda."
            }
            p {
                "Em "
                b { "@conta" }
                ", o usuário de quem tem conta: assim a pessoa também pode marcar o placar e tem as próprias estatísticas. Pode deixar em branco e vincular depois."
            }
            p {
                "No individual, a bolinha colorida escolhe a cor de cada um."
            }
        },
        Help::Room => rsx! {
            h3 { "O placar" }
            p {
                "Cada boi é uma cruz. Cada mão ganha pinta um braço; quem completar os 4 ganha o boi. "
                b { "Lambreta" }
                " é ganhar de 4 a 0."
            }
            h3 { "Marcar uma mão" }
            p {
                "Na primeira mão do boi, toque no nome de quem saiu (em "
                b { "Quem saiu?" }
                "). Nas outras o app já sabe quem começa e mostra em "
                b { "Sai:" }
                ", com a opção de trocar."
            }
            p {
                "No fim da mão, toque em "
                b { "Bateu" }
                " ou "
                b { "Ganhou no fechado" }
                " embaixo do nome de quem ganhou. Fechado sem vencedor: "
                b { "Fechado empatado" }
                ", e ninguém marca."
            }
            h3 { "Errou?" }
            p {
                b { "Desfazer última mão" }
                " apaga a última marcação."
            }
            h3 { "Jogadores" }
            p {
                b { "Editar" }
                " muda os jogadores e a ordem da roda, sempre entre um boi e outro."
            }
            h3 { "Histórico e estatísticas" }
            p {
                "No histórico, toque num dia para abrir e num boi para ver as mãos. Nas estatísticas, escolha o período: tudo, um dia, um mês ou um ano."
            }
            h3 { "Quem só assiste" }
            p {
                "Quem abre a sala pelo código vê o placar mudar sozinho, mas não marca."
            }
        },
        Help::Account => rsx! {
            h3 { "E-mail" }
            p {
                "Com um e-mail confirmado, você pode entrar com um código e trocar a senha se esquecer. Toque em "
                b { "Mandar código" }
                ", veja o código no e-mail (olhe também o spam) e digite aqui."
            }
            h3 { "Trocar a senha de alguém" }
            p {
                "Só aparece para quem cuida do app. Gera um link que vale por 1 hora e uma vez só, para mandar pelo WhatsApp a quem esqueceu a senha e não tem e-mail."
            }
            {theme_tip()}
        },
        Help::CodeLogin => rsx! {
            p {
                "Digite seu usuário ou e-mail e toque em "
                b { "Mandar código" }
                ". O código de 6 números chega no e-mail que você confirmou em "
                b { "Minha conta" }
                " e vale por 15 minutos. Não chegou? Olhe o spam ou peça outro."
            }
            p {
                "Sem e-mail confirmado não dá para receber o código. Peça um link a quem cuida do app."
            }
        },
        Help::NewPassword => rsx! {
            p {
                "Escolha a senha nova e toque em "
                b { "Trocar a senha e entrar" }
                ". O link só vale uma vez. Se ele não funcionar mais, peça outro a quem cuida do app."
            }
        },
    }
}

fn theme_tip() -> Element {
    rsx! {
        h3 { "Claro ou escuro" }
        p {
            "O botão ao lado do "
            b { "?" }
            " troca o visual: automático (segue o celular), claro ou escuro."
        }
    }
}

/// Convite discreto na primeira visita, apontando para o "?". Some ao tocar
/// em "Entendi" e não volta naquele aparelho (fica no `localStorage`).
#[component]
pub fn HelpInvite() -> Element {
    // Escondido até ler o `localStorage`, que só existe no navegador: quem já
    // viu o convite não o vê piscar.
    let mut show = use_signal(|| false);
    use_effect(move || {
        spawn(async move {
            let seen = document::eval(
                "try { return localStorage.getItem('convite-ajuda') === '1'; } catch (e) { return true; }",
            )
            .await
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            show.set(!seen);
        });
    });
    if !show() {
        return rsx! {};
    }
    rsx! {
        div { class: "invite",
            p {
                "Primeira vez aqui? Toque no "
                b { "?" }
                " no canto de cima para ver como funciona."
            }
            button {
                r#type: "button",
                class: "secondary",
                onclick: move |_| {
                    show.set(false);
                    document::eval("try { localStorage.setItem('convite-ajuda', '1'); } catch (e) {}");
                },
                "Entendi"
            }
        }
    }
}
