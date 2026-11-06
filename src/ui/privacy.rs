//! Política de privacidade. Endereço fixo (`/privacidade`): é o link
//! informado na Play Store. Mudou o que o app guarda? Atualize aqui e a data.

use super::{Help, TopBar};
use crate::Route;
use dioxus::prelude::*;

/// Quem responde pelo app e por onde falar com ele.
const RESPONSIBLE: &str = "Santa Maria Software";
const CONTACT: &str = "santa.maria.software.apps@gmail.com";
const UPDATED: &str = "5 de novembro de 2026";

#[component]
pub fn Privacy() -> Element {
    rsx! {
        main { class: "page",
            TopBar { help: Help::Privacy,
                Link { to: Route::Home {}, "← Início" }
            }
            section { class: "card help",
                h2 { "Política de privacidade" }
                p { class: "muted", "Atualizada em {UPDATED}." }
                p {
                    "O Placar de Dominó é um placar para jogar em família, feito pela {RESPONSIBLE}. Esta página explica o que o app guarda, para quê e como apagar."
                }

                h3 { "O que o app guarda" }
                ul {
                    li {
                        b { "Conta: " }
                        "usuário, nome que aparece no placar e senha. A senha é guardada embaralhada (hash), e ninguém consegue lê-la."
                    }
                    li {
                        b { "E-mail, se você confirmar um: " }
                        "serve só para mandar códigos de entrada e de troca de senha."
                    }
                    li {
                        b { "Jogos: " }
                        "salas, nomes dos jogadores (digitados por quem cria a sala), mãos e bois, com data e hora."
                    }
                    li {
                        b { "Sessão: " }
                        "um cookie que mantém você conectado. Não há cookies de propaganda nem de rastreamento."
                    }
                    li {
                        b { "No seu aparelho: " }
                        "a escolha de tema claro ou escuro e se você já viu o convite da ajuda."
                    }
                    li {
                        b { "Registros técnicos do servidor: " }
                        "endereço IP e páginas pedidas, guardados pelo provedor de hospedagem por um tempo limitado, para corrigir erros e evitar abuso."
                    }
                }

                h3 { "O que o app não faz" }
                p {
                    "Não mostra propaganda, não vende nem compartilha seus dados com ninguém para marketing e não usa ferramentas de análise ou rastreamento."
                }

                h3 { "Quem vê o quê" }
                p {
                    "Qualquer pessoa com o código de uma sala vê o placar, o histórico e as estatísticas dela, com os nomes dos jogadores. Seu e-mail e sua senha não aparecem para ninguém."
                }

                h3 { "Onde os dados ficam" }
                p {
                    "Em serviços que o app usa para funcionar, todos com servidores em São Paulo, exceto o e-mail: o Google Cloud (hospedagem), o Turso (banco de dados) e o Gmail (envio dos códigos por e-mail). A conexão é sempre criptografada (HTTPS)."
                }

                h3 { "Como apagar seus dados" }
                p {
                    "Em "
                    b { "Minha conta" }
                    ", toque em "
                    b { "Excluir minha conta" }
                    ", ou abra "
                    Link { to: Route::DeleteAccount {}, "a página de exclusão" }
                    ". A conta, o e-mail, a senha e as sessões são apagados na hora. Nas salas em que você jogou, seu nome continua no placar e no histórico, sem ligação com a conta, porque o histórico é de todos que jogaram. As salas que você criou passam para outro jogador com conta ou, se não houver nenhum, são apagadas com o histórico."
                }

                h3 { "Seus direitos" }
                p {
                    "Pela Lei Geral de Proteção de Dados (LGPD), você pode pedir para ver, corrigir ou apagar seus dados. Fale com a gente pelo e-mail "
                    a { href: "mailto:{CONTACT}", "{CONTACT}" }
                    "."
                }

                h3 { "Crianças" }
                p {
                    "O app não pede idade nem é feito para crianças criarem conta sozinhas. Uma criança pode aparecer como jogador numa sala, só com o nome que um adulto digitou."
                }
            }
        }
    }
}
