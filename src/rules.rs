//! Regras do placar, sem I/O. Rodam igual no servidor e no navegador.
//!
//! - Cada mão vencida (batida ou fechado) vale 1 bolinha para o lado vencedor.
//! - Fechado empatado não dá bolinha a ninguém.
//! - O boi acaba quando um lado completa a cruz: 4 bolinhas.
//! - Lambreta: o perdedor termina o boi com 0.
//! - Lambreta reversa: o perdedor marcou, mas o vencedor fez as 4 bolinhas dele em sequência.
//! - Saída: enquanto nenhuma mão do boi tiver vencedor, sai quem tem a maior carroça
//!   (o app pergunta). Depois disso, a saída passa para o próximo jogador da roda.

use crate::model::{Hand, Mode, Outcome};
use serde::{Deserialize, Serialize};

/// Bolinhas da cruz: uma em cada braço.
pub const POINTS_TO_WIN: u32 = 4;

/// Lado (jogador ou dupla) de um assento. Nas duplas, os parceiros ficam frente a frente:
/// assentos 0 e 2 são a dupla A, assentos 1 e 3 são a dupla B.
pub fn side_of(mode: Mode, seat: usize) -> usize {
    match mode {
        Mode::Individual => seat,
        Mode::Duplas => seat % 2,
    }
}

pub fn side_count(mode: Mode, players: usize) -> usize {
    match mode {
        Mode::Individual => players,
        Mode::Duplas => 2,
    }
}

pub fn valid_player_count(mode: Mode, players: usize) -> bool {
    match mode {
        Mode::Individual => (2..=4).contains(&players),
        Mode::Duplas => players == 4,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideResult {
    Venceu,
    Perdeu,
    Lambreta,
    LambretaReversa,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoiState {
    pub scores: Vec<u32>,
    pub winner: Option<usize>,
    /// Resultado de cada lado, preenchido só quando o boi acaba.
    pub results: Vec<Option<SideResult>>,
    /// Quem sai na próxima mão. `None` significa "regra da carroça": perguntar quem saiu.
    pub next_starter: Option<usize>,
}

impl BoiState {
    pub fn finished(&self) -> bool {
        self.winner.is_some()
    }
}

pub fn evaluate(mode: Mode, players: usize, hands: &[Hand]) -> BoiState {
    let sides = side_count(mode, players);
    let mut scores = vec![0u32; sides];
    let mut winner = None;
    for hand in hands {
        if let Some(side) = hand.winner_side {
            if side < sides {
                scores[side] += 1;
                if scores[side] >= POINTS_TO_WIN && winner.is_none() {
                    winner = Some(side);
                }
            }
        }
    }

    let results = match winner {
        None => vec![None; sides],
        Some(w) => {
            let swept = winner_swept(hands, w);
            (0..sides)
                .map(|side| {
                    Some(if side == w {
                        SideResult::Venceu
                    } else if scores[side] == 0 {
                        SideResult::Lambreta
                    } else if swept {
                        SideResult::LambretaReversa
                    } else {
                        SideResult::Perdeu
                    })
                })
                .collect()
        }
    };

    let next_starter = if winner.is_some() || players == 0 {
        None
    } else if hands.iter().any(|h| h.winner_side.is_some()) {
        hands.last().map(|h| (h.starter_seat + 1) % players)
    } else {
        None
    };

    BoiState {
        scores,
        winner,
        results,
        next_starter,
    }
}

/// O vencedor fez todas as bolinhas dele depois da última mão ganha por outro lado?
/// Empates não interrompem a sequência.
fn winner_swept(hands: &[Hand], winner: usize) -> bool {
    let mut winner_points = 0;
    let mut points_at_last_other_win = 0;
    for hand in hands {
        match hand.winner_side {
            Some(side) if side == winner => winner_points += 1,
            Some(_) => points_at_last_other_win = winner_points,
            None => {}
        }
    }
    points_at_last_other_win == 0
}

/// Valida uma mão nova antes de gravar.
pub fn validate_hand(
    mode: Mode,
    players: usize,
    hands: &[Hand],
    starter_seat: usize,
    outcome: Outcome,
    winner_side: Option<usize>,
) -> Result<(), String> {
    if evaluate(mode, players, hands).finished() {
        return Err("Este boi já acabou. Comece um boi novo.".into());
    }
    if starter_seat >= players {
        return Err("Jogador de saída inválido.".into());
    }
    match (outcome, winner_side) {
        (Outcome::Empate, None) => Ok(()),
        (Outcome::Empate, Some(_)) => Err("Empate não tem vencedor.".into()),
        (_, None) => Err("Informe quem ganhou a mão.".into()),
        (_, Some(side)) if side >= side_count(mode, players) => Err("Vencedor inválido.".into()),
        _ => Ok(()),
    }
}

/// Estatísticas de um jogador. Nas duplas, a vitória da dupla conta para os dois parceiros.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerStats {
    pub player_id: i64,
    pub maos_jogadas: u32,
    pub maos_ganhas: u32,
    pub batidas: u32,
    pub fechados: u32,
    pub bois_jogados: u32,
    pub bois_ganhos: u32,
    pub lambretas_aplicadas: u32,
    pub lambretas_sofridas: u32,
    pub reversas_aplicadas: u32,
    pub reversas_sofridas: u32,
}

impl PlayerStats {
    pub fn add(&mut self, other: &PlayerStats) {
        self.maos_jogadas += other.maos_jogadas;
        self.maos_ganhas += other.maos_ganhas;
        self.batidas += other.batidas;
        self.fechados += other.fechados;
        self.bois_jogados += other.bois_jogados;
        self.bois_ganhos += other.bois_ganhos;
        self.lambretas_aplicadas += other.lambretas_aplicadas;
        self.lambretas_sofridas += other.lambretas_sofridas;
        self.reversas_aplicadas += other.reversas_aplicadas;
        self.reversas_sofridas += other.reversas_sofridas;
    }
}

/// Soma as estatísticas de uma lista de bois. `seating` de cada boi é a lista de
/// ids de jogador na ordem dos assentos.
pub fn compute_stats<'a>(
    bois: impl IntoIterator<Item = (Mode, &'a [i64], &'a [Hand])>,
) -> Vec<PlayerStats> {
    let mut stats: Vec<PlayerStats> = Vec::new();
    let entry = |stats: &mut Vec<PlayerStats>, id: i64| -> usize {
        match stats.iter().position(|s| s.player_id == id) {
            Some(i) => i,
            None => {
                stats.push(PlayerStats {
                    player_id: id,
                    ..Default::default()
                });
                stats.len() - 1
            }
        }
    };

    for (mode, seating, hands) in bois {
        let state = evaluate(mode, seating.len(), hands);
        for (seat, &player) in seating.iter().enumerate() {
            let i = entry(&mut stats, player);
            let side = side_of(mode, seat);
            let s = &mut stats[i];
            if state.finished() {
                s.bois_jogados += 1;
            }
            s.maos_jogadas += hands.len() as u32;
            for hand in hands.iter().filter(|h| h.winner_side == Some(side)) {
                s.maos_ganhas += 1;
                match hand.outcome {
                    Outcome::Batida => s.batidas += 1,
                    Outcome::Fechado => s.fechados += 1,
                    Outcome::Empate => {}
                }
            }
            let Some(winner) = state.winner else { continue };
            if side == winner {
                s.bois_ganhos += 1;
                for result in state.results.iter().flatten() {
                    match result {
                        SideResult::Lambreta => s.lambretas_aplicadas += 1,
                        SideResult::LambretaReversa => s.reversas_aplicadas += 1,
                        _ => {}
                    }
                }
            } else {
                match state.results[side] {
                    Some(SideResult::Lambreta) => s.lambretas_sofridas += 1,
                    Some(SideResult::LambretaReversa) => s.reversas_sofridas += 1,
                    _ => {}
                }
            }
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand(starter: usize, winner: Option<usize>) -> Hand {
        Hand {
            starter_seat: starter,
            outcome: if winner.is_some() {
                Outcome::Batida
            } else {
                Outcome::Empate
            },
            winner_side: winner,
        }
    }

    /// Mãos a partir de uma sequência de vencedores, com a saída girando a partir do assento 0.
    fn hands(winners: &[Option<usize>], players: usize) -> Vec<Hand> {
        winners
            .iter()
            .enumerate()
            .map(|(i, w)| hand(i % players, *w))
            .collect()
    }

    #[test]
    fn lambreta_4_0() {
        let s = evaluate(Mode::Individual, 2, &hands(&[Some(0); 4], 2));
        assert_eq!(s.scores, vec![4, 0]);
        assert_eq!(s.winner, Some(0));
        assert_eq!(s.results[1], Some(SideResult::Lambreta));
    }

    #[test]
    fn lambreta_reversa() {
        // Perdendo de 0-3, vira para 4-3.
        let w = [
            Some(1),
            Some(1),
            Some(1),
            Some(0),
            Some(0),
            Some(0),
            Some(0),
        ];
        let s = evaluate(Mode::Individual, 2, &hands(&w, 2));
        assert_eq!(s.scores, vec![4, 3]);
        assert_eq!(s.results[1], Some(SideResult::LambretaReversa));
    }

    #[test]
    fn empate_nao_quebra_a_sequencia() {
        let w = [Some(1), Some(0), None, Some(0), Some(0), Some(0)];
        let s = evaluate(Mode::Individual, 2, &hands(&w, 2));
        assert_eq!(s.results[1], Some(SideResult::LambretaReversa));
    }

    #[test]
    fn vitoria_normal() {
        let w = [Some(0), Some(1), Some(0), Some(0), Some(0)];
        let s = evaluate(Mode::Individual, 2, &hands(&w, 2));
        assert_eq!(s.scores, vec![4, 1]);
        assert_eq!(s.results[1], Some(SideResult::Perdeu));
    }

    #[test]
    fn individual_varios_lambretados() {
        // 4-2-0-0 com as 4 do vencedor em sequência: reversa no jogador 1, lambreta nos 2 e 3.
        let w = [Some(1), Some(1), Some(0), Some(0), Some(0), Some(0)];
        let s = evaluate(Mode::Individual, 4, &hands(&w, 4));
        assert_eq!(
            s.results,
            vec![
                Some(SideResult::Venceu),
                Some(SideResult::LambretaReversa),
                Some(SideResult::Lambreta),
                Some(SideResult::Lambreta),
            ]
        );
    }

    #[test]
    fn saida_regra_da_carroca_enquanto_ninguem_ganha() {
        assert_eq!(evaluate(Mode::Duplas, 4, &[]).next_starter, None);
        // Dois empates seguidos: continua perguntando.
        let h = vec![hand(2, None), hand(1, None)];
        assert_eq!(evaluate(Mode::Duplas, 4, &h).next_starter, None);
        // Primeira mão com vencedor: a saída passa para o próximo da roda.
        let h = vec![hand(2, None), hand(3, Some(1))];
        assert_eq!(evaluate(Mode::Duplas, 4, &h).next_starter, Some(0));
        // Depois, empate também gira a saída.
        let h = vec![hand(3, Some(1)), hand(0, None)];
        assert_eq!(evaluate(Mode::Duplas, 4, &h).next_starter, Some(1));
    }

    #[test]
    fn duplas_lados() {
        assert_eq!(side_of(Mode::Duplas, 2), 0);
        assert_eq!(side_of(Mode::Duplas, 3), 1);
        let s = evaluate(Mode::Duplas, 4, &hands(&[Some(1); 4], 4));
        assert_eq!(s.winner, Some(1));
        assert_eq!(s.results[0], Some(SideResult::Lambreta));
    }

    #[test]
    fn validacao() {
        let done = hands(&[Some(0); 4], 2);
        assert!(validate_hand(Mode::Individual, 2, &done, 0, Outcome::Batida, Some(0)).is_err());
        assert!(validate_hand(Mode::Individual, 2, &[], 0, Outcome::Empate, None).is_ok());
        assert!(validate_hand(Mode::Individual, 2, &[], 0, Outcome::Empate, Some(0)).is_err());
        assert!(validate_hand(Mode::Duplas, 4, &[], 0, Outcome::Fechado, Some(2)).is_err());
    }

    #[test]
    fn estatisticas_duplas() {
        let seating = [10, 20, 11, 21];
        let h = hands(&[Some(0); 4], 4);
        let stats = compute_stats([(Mode::Duplas, &seating[..], &h[..])]);
        let get = |id| stats.iter().find(|s| s.player_id == id).unwrap();
        assert_eq!(get(10).bois_ganhos, 1);
        assert_eq!(get(11).lambretas_aplicadas, 1);
        assert_eq!(get(20).lambretas_sofridas, 1);
        assert_eq!(get(21).maos_ganhas, 0);
        assert_eq!(get(10).maos_ganhas, 4);
    }

    #[test]
    fn boi_em_andamento_nao_conta_como_jogado() {
        let seating = [1, 2];
        let h = hands(&[Some(0)], 2);
        let stats = compute_stats([(Mode::Individual, &seating[..], &h[..])]);
        assert_eq!(stats[0].bois_jogados, 0);
        assert_eq!(stats[0].maos_ganhas, 1);
    }
}
