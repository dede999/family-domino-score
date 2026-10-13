//! Tipos do domínio: modo de jogo, mão, boi, jogador, duplas e cores.
//! Rodam igual no servidor e no navegador.

use crate::{palette, rules};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Individual,
    Duplas,
}

#[cfg_attr(not(feature = "server"), allow(dead_code))]
impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Individual => "individual",
            Mode::Duplas => "duplas",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "individual" => Some(Mode::Individual),
            "duplas" => Some(Mode::Duplas),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Batida,
    Fechado,
    Empate,
}

#[cfg_attr(not(feature = "server"), allow(dead_code))]
impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Batida => "batida",
            Outcome::Fechado => "fechado",
            Outcome::Empate => "empate",
        }
    }

    pub fn parse(s: &str) -> Option<Outcome> {
        match s {
            "batida" => Some(Outcome::Batida),
            "fechado" => Some(Outcome::Fechado),
            "empate" => Some(Outcome::Empate),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hand {
    pub starter_seat: usize,
    pub outcome: Outcome,
    pub winner_side: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: i64,
    pub name: String,
    /// Conta vinculada. `None` para jogador avulso.
    pub username: Option<String>,
    /// Índice na paleta. Só vale no modo individual; nas duplas a cor é da dupla.
    pub color: u8,
}

/// Cores de uma dupla. A dupla é o par de jogadores, sem ordem: guardada com
/// o menor id primeiro, vale em qualquer boi em que os dois joguem juntos.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairColors {
    pub players: (i64, i64),
    pub primary: u8,
    /// Reserva: usada quando a outra dupla do boi tem a mesma primária e
    /// escolheu antes. Sempre diferente da primária.
    pub secondary: u8,
    /// Quando a primária foi escolhida (segundos desde 1970).
    pub chosen_at: i64,
}

pub fn pair_key(a: i64, b: i64) -> (i64, i64) {
    (a.min(b), a.max(b))
}

/// Ids dos jogadores de cada lado, na ordem dos lados.
pub fn sides_of(mode: Mode, seating: &[i64]) -> Vec<Vec<i64>> {
    let mut sides = vec![Vec::new(); rules::side_count(mode, seating.len())];
    for (seat, &id) in seating.iter().enumerate() {
        sides[rules::side_of(mode, seat)].push(id);
    }
    sides
}

/// Cor de cada lado para uma roda de jogadores.
///
/// Individual: a cor de cada jogador. Duplas: a primária de cada dupla; se as
/// duas tiverem a mesma, quem a escolheu por último usa a secundária. Dupla
/// que ainda não tem cor recebe a primeira livre.
pub fn resolve_side_colors(
    mode: Mode,
    seating: &[i64],
    players: &[Player],
    pairs: &[PairColors],
) -> Vec<u8> {
    let sides = sides_of(mode, seating);
    match mode {
        Mode::Individual => sides
            .iter()
            .map(|ids| {
                ids.first()
                    .and_then(|id| players.iter().find(|p| p.id == *id))
                    .map_or(0, |p| p.color)
            })
            .collect(),
        Mode::Duplas => {
            let found: Vec<Option<&PairColors>> = sides
                .iter()
                .map(|ids| match ids[..] {
                    [a, b] => pairs.iter().find(|p| p.players == pair_key(a, b)),
                    _ => None,
                })
                .collect();
            match (found[0], found[1]) {
                (Some(a), Some(b)) if a.primary == b.primary => {
                    if b.chosen_at >= a.chosen_at {
                        vec![a.primary, b.secondary]
                    } else {
                        vec![a.secondary, b.primary]
                    }
                }
                (Some(a), Some(b)) => vec![a.primary, b.primary],
                (Some(a), None) => vec![a.primary, palette::first_free(&[a.primary])],
                (None, Some(b)) => vec![palette::first_free(&[b.primary]), b.primary],
                (None, None) => vec![0, 1],
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoiView {
    pub id: i64,
    pub number: u32,
    /// Ids dos jogadores na ordem dos assentos, congelada no início do boi.
    pub seating: Vec<i64>,
    pub hands: Vec<Hand>,
    /// Cor de cada lado, congelada no início do boi. Vazio em bois antigos.
    pub colors: Vec<u8>,
    /// Início do boi em segundos desde 1970 (UTC).
    pub started_at: i64,
}

/// Chave para ordenar nomes: sem acento e sem diferença de maiúsculas,
/// para "Álvaro" ficar antes de "Bruno".
pub fn sort_key(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: i64, b: i64, primary: u8, secondary: u8, chosen_at: i64) -> PairColors {
        PairColors {
            players: pair_key(a, b),
            primary,
            secondary,
            chosen_at,
        }
    }

    // Roda das duplas: assentos 0 e 2 são o lado A, 1 e 3 o lado B.
    const RODA: [i64; 4] = [1, 2, 3, 4];

    #[test]
    fn duplas_com_cores_diferentes() {
        let pairs = [pair(1, 3, 5, 0, 10), pair(2, 4, 2, 0, 20)];
        assert_eq!(
            resolve_side_colors(Mode::Duplas, &RODA, &[], &pairs),
            vec![5, 2]
        );
    }

    #[test]
    fn mesma_primaria_quem_escolheu_por_ultimo_usa_a_reserva() {
        let pairs = [pair(3, 1, 5, 6, 10), pair(2, 4, 5, 7, 20)];
        assert_eq!(
            resolve_side_colors(Mode::Duplas, &RODA, &[], &pairs),
            vec![5, 7]
        );
        let pairs = [pair(1, 3, 5, 6, 30), pair(2, 4, 5, 7, 20)];
        assert_eq!(
            resolve_side_colors(Mode::Duplas, &RODA, &[], &pairs),
            vec![6, 5]
        );
    }

    #[test]
    fn dupla_nova_recebe_cor_livre() {
        let pairs = [pair(2, 4, 0, 1, 10)];
        assert_eq!(
            resolve_side_colors(Mode::Duplas, &RODA, &[], &pairs),
            vec![1, 0]
        );
        assert_eq!(
            resolve_side_colors(Mode::Duplas, &RODA, &[], &[]),
            vec![0, 1]
        );
    }

    #[test]
    fn individual_usa_a_cor_de_cada_jogador() {
        let p = |id, color| Player {
            id,
            name: String::new(),
            username: None,
            color,
        };
        let players = [p(1, 3), p(2, 7), p(3, 0)];
        assert_eq!(
            resolve_side_colors(Mode::Individual, &[2, 3, 1], &players, &[]),
            vec![7, 0, 3]
        );
    }

    #[test]
    fn nomes_em_ordem_alfabetica_sem_acento() {
        let mut names = vec!["Bruno", "Álvaro", "carlos", "Ana"];
        names.sort_by_key(|n| sort_key(n));
        assert_eq!(names, ["Álvaro", "Ana", "Bruno", "carlos"]);
    }
}
