//! Regras do placar, sem I/O. Rodam igual no servidor e no navegador.
//!
//! - Cada mão vencida (batida ou fechado) vale 1 bolinha para o lado vencedor.
//! - Fechado empatado não dá bolinha a ninguém.
//! - O boi acaba quando um lado completa a cruz: 4 bolinhas.
//! - Lambreta: o perdedor termina o boi com 0.
//! - Lambreta reversa: o perdedor marcou, mas o vencedor fez as 4 bolinhas dele em sequência.
//! - Saída: enquanto nenhuma mão do boi tiver vencedor, sai quem tem a maior carroça
//!   (o app pergunta). Depois disso, a saída passa para o próximo jogador da roda.

use crate::model::{BoiView, Hand, Mode, Outcome, pair_key, sides_of};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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
        if let Some(side) = hand.winner_side
            && side < sides
        {
            scores[side] += 1;
            if scores[side] >= POINTS_TO_WIN && winner.is_none() {
                winner = Some(side);
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

/// O que decide o destaque de um período.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub bois_jogados: u32,
    pub bois_ganhos: u32,
    pub maos_jogadas: u32,
    pub maos_ganhas: u32,
}

impl Record {
    /// Mais bois ganhos; empate vai para o maior % de mãos ganhas.
    pub fn cmp_best(&self, other: &Record) -> Ordering {
        let share = |a: &Record, b: &Record| u64::from(a.maos_ganhas) * u64::from(b.maos_jogadas);
        self.bois_ganhos
            .cmp(&other.bois_ganhos)
            .then_with(|| share(self, other).cmp(&share(other, self)))
    }

    /// % de mãos ganhas, arredondado.
    pub fn hand_share(&self) -> u32 {
        (self.maos_ganhas * 100 + self.maos_jogadas / 2)
            .checked_div(self.maos_jogadas)
            .unwrap_or(0)
    }
}

impl PlayerStats {
    pub fn record(&self) -> Record {
        Record {
            bois_jogados: self.bois_jogados,
            bois_ganhos: self.bois_ganhos,
            maos_jogadas: self.maos_jogadas,
            maos_ganhas: self.maos_ganhas,
        }
    }
}

/// Desempenho de uma dupla (os dois ids em ordem crescente).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairRecord {
    pub players: (i64, i64),
    pub record: Record,
}

/// Desempenho de cada dupla que jogou. Vazio fora do modo duplas.
pub fn pair_records<'a>(
    mode: Mode,
    bois: impl IntoIterator<Item = &'a BoiView>,
) -> Vec<PairRecord> {
    let mut pairs: Vec<PairRecord> = Vec::new();
    if mode != Mode::Duplas {
        return pairs;
    }
    for boi in bois {
        let state = evaluate(mode, boi.seating.len(), &boi.hands);
        for (side, ids) in sides_of(mode, &boi.seating).iter().enumerate() {
            let [a, b] = ids[..] else { continue };
            let key = pair_key(a, b);
            let i = match pairs.iter().position(|p| p.players == key) {
                Some(i) => i,
                None => {
                    pairs.push(PairRecord {
                        players: key,
                        record: Record::default(),
                    });
                    pairs.len() - 1
                }
            };
            let r = &mut pairs[i].record;
            r.maos_jogadas += boi.hands.len() as u32;
            r.maos_ganhas += boi
                .hands
                .iter()
                .filter(|h| h.winner_side == Some(side))
                .count() as u32;
            if let Some(w) = state.winner {
                r.bois_jogados += 1;
                if w == side {
                    r.bois_ganhos += 1;
                }
            }
        }
    }
    pairs
}

/// Os melhores de uma lista pelo critério do destaque (mais de um se
/// empatarem em tudo). Vazio se ninguém ganhou boi.
pub fn best<T>(items: &[T], record: impl Fn(&T) -> Record) -> Vec<&T> {
    let Some(top) = items.iter().map(&record).max_by(|a, b| a.cmp_best(b)) else {
        return Vec::new();
    };
    if top.bois_ganhos == 0 {
        return Vec::new();
    }
    items
        .iter()
        .filter(|i| record(i).cmp_best(&top) == Ordering::Equal)
        .collect()
}

/// Com menos bois terminados que isso, a sala ainda não mostra as viradas.
pub const MIN_BOIS_FOR_COMEBACKS: usize = 5;

/// Quantas vezes um lado esteve atrás num placar e quantas vezes virou.
/// `behind` é o placar de quem está atrás; `leader`, o de quem está na frente
/// (no individual, o maior placar entre os outros).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comeback {
    pub behind: u32,
    pub leader: u32,
    pub times: u32,
    pub turned: u32,
}

/// Placar de cada lado em relação ao líder: `Some((meu, líder))` se o lado
/// está atrás, `None` se está na frente ou empatado com o líder.
pub fn deficits(scores: &[u32]) -> Vec<Option<(u32, u32)>> {
    (0..scores.len())
        .map(|side| {
            let leader = (0..scores.len())
                .filter(|&o| o != side)
                .map(|o| scores[o])
                .max()?;
            (scores[side] < leader).then_some((scores[side], leader))
        })
        .collect()
}

/// Viradas da sala: para cada placar com alguém atrás, quantas vezes um
/// lado passou por ele num boi terminado e quantas vezes esse lado levou o
/// boi. Cada lado conta cada placar uma vez por boi.
pub fn comebacks<'a>(mode: Mode, bois: impl IntoIterator<Item = &'a BoiView>) -> Vec<Comeback> {
    let mut table: Vec<Comeback> = Vec::new();
    for boi in bois {
        let sides = side_count(mode, boi.seating.len());
        let Some(winner) = evaluate(mode, boi.seating.len(), &boi.hands).winner else {
            continue;
        };
        let mut scores = vec![0u32; sides];
        let mut seen: Vec<(usize, (u32, u32))> = Vec::new();
        for hand in &boi.hands {
            let Some(w) = hand.winner_side.filter(|&w| w < sides) else {
                continue;
            };
            scores[w] += 1;
            if scores[w] >= POINTS_TO_WIN {
                break;
            }
            for (side, d) in deficits(&scores).into_iter().enumerate() {
                if let Some(d) = d
                    && !seen.contains(&(side, d))
                {
                    seen.push((side, d));
                }
            }
        }
        for (side, (behind, leader)) in seen {
            let i = match table
                .iter()
                .position(|c| (c.behind, c.leader) == (behind, leader))
            {
                Some(i) => i,
                None => {
                    table.push(Comeback {
                        behind,
                        leader,
                        times: 0,
                        turned: 0,
                    });
                    table.len() - 1
                }
            };
            table[i].times += 1;
            if side == winner {
                table[i].turned += 1;
            }
        }
    }
    table
}

/// Um acontecimento marcante: quem fez, contra quem (quando houver) e em
/// que boi. Os lados são listas de ids de jogador.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moment {
    pub boi_number: u32,
    pub started_at: i64,
    pub by: Vec<i64>,
    pub against: Vec<Vec<i64>>,
}

/// As primeiras vezes de um período (pensado para o ano).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Firsts {
    /// Quem saiu na primeira mão.
    pub saida: Option<Moment>,
    /// Quem ganhou a primeira mão (empates não contam).
    pub mao: Option<Moment>,
    pub boi: Option<Moment>,
    /// `against`: só quem levou a lambreta.
    pub lambreta: Option<Moment>,
    pub reversa: Option<Moment>,
}

/// Primeiras vezes numa lista de bois, do mais antigo para o mais recente.
pub fn firsts<'a>(mode: Mode, bois: impl IntoIterator<Item = &'a BoiView>) -> Firsts {
    let mut f = Firsts::default();
    for boi in bois {
        let sides = sides_of(mode, &boi.seating);
        let moment = |by: Vec<i64>, against: Vec<Vec<i64>>| Moment {
            boi_number: boi.number,
            started_at: boi.started_at,
            by,
            against,
        };
        let others = |w: usize| -> Vec<Vec<i64>> {
            (0..sides.len())
                .filter(|&s| s != w)
                .map(|s| sides[s].clone())
                .collect()
        };
        if let Some(h) = boi.hands.first()
            && f.saida.is_none()
        {
            f.saida = boi
                .seating
                .get(h.starter_seat)
                .map(|&id| moment(vec![id], Vec::new()));
        }
        if f.mao.is_none()
            && let Some(w) = boi.hands.iter().find_map(|h| h.winner_side)
        {
            f.mao = Some(moment(sides[w].clone(), others(w)));
        }
        let state = evaluate(mode, boi.seating.len(), &boi.hands);
        let Some(w) = state.winner else { continue };
        if f.boi.is_none() {
            f.boi = Some(moment(sides[w].clone(), others(w)));
        }
        let hit = |kind: SideResult| -> Vec<Vec<i64>> {
            (0..sides.len())
                .filter(|&s| state.results[s] == Some(kind))
                .map(|s| sides[s].clone())
                .collect()
        };
        for (slot, kind) in [
            (&mut f.lambreta, SideResult::Lambreta),
            (&mut f.reversa, SideResult::LambretaReversa),
        ] {
            let victims = hit(kind);
            if slot.is_none() && !victims.is_empty() {
                *slot = Some(moment(sides[w].clone(), victims));
            }
        }
    }
    f
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

    #[test]
    fn primeiras_vezes() {
        let boi = |number: u32, winners: &[Option<usize>]| BoiView {
            id: i64::from(number),
            number,
            seating: vec![1, 2, 3, 4],
            hands: hands(winners, 4),
            colors: Vec::new(),
            started_at: i64::from(number),
        };
        let bois = [
            // Boi 1: empate, depois a dupla B (2 e 4) leva por 4 a 2.
            boi(
                1,
                &[None, Some(1), Some(0), Some(1), Some(0), Some(1), Some(1)],
            ),
            // Boi 2: a dupla A faz 4 a 0.
            boi(2, &[Some(0); 4]),
            // Boi 3: B marca 1, A faz 4 seguidas.
            boi(3, &[Some(1), Some(0), Some(0), Some(0), Some(0)]),
        ];
        let f = firsts(Mode::Duplas, &bois);
        assert_eq!(f.saida.unwrap().by, vec![1]);
        let mao = f.mao.unwrap();
        assert_eq!((mao.boi_number, mao.by), (1, vec![2, 4]));
        assert_eq!(f.boi.unwrap().by, vec![2, 4]);
        let lamb = f.lambreta.unwrap();
        assert_eq!(
            (lamb.boi_number, lamb.by, lamb.against),
            (2, vec![1, 3], vec![vec![2, 4]])
        );
        assert_eq!(f.reversa.unwrap().boi_number, 3);
        assert_eq!(firsts(Mode::Duplas, &[]), Firsts::default());
    }

    #[test]
    fn destaque_por_bois_e_depois_por_maos() {
        let r = |bg, mg, mj| Record {
            bois_jogados: 3,
            bois_ganhos: bg,
            maos_jogadas: mj,
            maos_ganhas: mg,
        };
        // Mais bois ganha, mesmo com % de mãos menor.
        let list = [r(1, 9, 10), r(2, 8, 20), r(2, 10, 20)];
        assert_eq!(best(&list, |x| *x), vec![&list[2]]);
        // 4 de 8 empata com 8 de 16: os dois são destaque.
        let list = [r(2, 4, 8), r(2, 8, 16), r(1, 9, 9)];
        assert_eq!(best(&list, |x| *x).len(), 2);
        // Ninguém ganhou boi: sem destaque.
        assert!(best(&[r(0, 3, 5)], |x| *x).is_empty());
        assert_eq!(r(0, 2, 3).hand_share(), 67);
    }

    #[test]
    fn duplas_trocam_e_cada_par_tem_seu_registro() {
        let boi = |seating: Vec<i64>, winners: &[Option<usize>]| BoiView {
            id: 1,
            number: 1,
            seating,
            hands: hands(winners, 4),
            colors: Vec::new(),
            started_at: 0,
        };
        let bois = [
            boi(vec![1, 2, 3, 4], &[Some(0); 4]),
            boi(
                vec![1, 3, 2, 4],
                &[Some(1), Some(0), Some(0), Some(0), Some(0)],
            ),
        ];
        let pairs = pair_records(Mode::Duplas, &bois);
        let get = |a, b| pairs.iter().find(|p| p.players == (a, b)).unwrap().record;
        assert_eq!(get(1, 3).bois_ganhos, 1);
        assert_eq!((get(1, 2).bois_ganhos, get(1, 2).maos_ganhas), (1, 4));
        assert_eq!((get(3, 4).bois_jogados, get(3, 4).maos_ganhas), (1, 1));
        assert!(pair_records(Mode::Individual, &bois).is_empty());
    }

    #[test]
    fn viradas_contam_cada_placar_uma_vez_por_boi() {
        let boi = |winners: &[Option<usize>]| BoiView {
            id: 1,
            number: 1,
            seating: vec![1, 2, 3, 4],
            hands: hands(winners, 4),
            colors: Vec::new(),
            started_at: 0,
        };
        let a = Some(0);
        let b = Some(1);
        let bois = [
            // B abre 3 a 1 e A vira (com um empate no meio, que não conta).
            boi(&[b, b, a, b, None, a, a, a]),
            // B abre 1 a 0 e leva.
            boi(&[b, a, b, b, b]),
            // Boi sem fim: não conta.
            boi(&[b, b, b]),
        ];
        let table = comebacks(Mode::Duplas, &bois);
        let get = |behind, leader| {
            table
                .iter()
                .find(|c| (c.behind, c.leader) == (behind, leader))
                .map(|c| (c.times, c.turned))
        };
        // 1 a 3 aconteceu nos dois bois e só o primeiro virou.
        assert_eq!(get(1, 3), Some((2, 1)));
        assert_eq!(get(0, 1), Some((2, 1)));
        assert_eq!(get(0, 2), Some((1, 1)));
        // O 3 a 4 final não é "estar atrás": o boi já acabou.
        assert_eq!(get(3, 4), None);
        // No individual, quem empata com o líder não está atrás.
        assert_eq!(deficits(&[3, 3, 1]), vec![None, None, Some((1, 3))]);
    }
}
