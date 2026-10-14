//! O que viaja entre o servidor e a tela: a sala pronta para mostrar,
//! páginas do histórico, relatórios de período e dados da conta.

use crate::model::{BoiView, Mode, PairColors, Player, resolve_side_colors, sides_of, sort_key};
use crate::period::Period;
use crate::rules::{self, BoiState, Comeback, Firsts, PairRecord, PlayerStats};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub display_name: String,
}

/// Dados da própria conta, para a tela "Minha conta".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub user: UserInfo,
    /// E-mail confirmado, para receber códigos de entrada.
    pub email: Option<String>,
    /// Pode gerar links de troca de senha para os outros.
    pub is_admin: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomSummary {
    pub code: String,
    pub name: String,
    pub mode: Mode,
}

/// Jogador na criação ou edição de uma sala.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    /// `None` para jogador novo.
    pub id: Option<i64>,
    pub name: String,
    /// Vincular a uma conta existente pelo nome de usuário.
    pub username: Option<String>,
    /// Índice na paleta (modo individual). `None` = o servidor escolhe uma livre.
    pub color: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomView {
    pub code: String,
    pub name: String,
    pub mode: Mode,
    pub owner: String,
    pub is_owner: bool,
    pub can_edit: bool,
    /// Ordem atual dos jogadores na roda.
    pub players: Vec<Player>,
    /// O boi mais recente, terminado ou não.
    pub current: Option<BoiView>,
    /// Primeira página do histórico: os dias mais recentes com boi terminado,
    /// só com o resumo. As outras vêm de `api::room_history`, e os bois de
    /// cada dia, de `api::room_day`.
    pub history: DaysPage,
    /// Estatísticas de todos os bois da sala, calculadas no servidor.
    pub stats: Vec<PlayerStats>,
    /// Hoje no fuso de Brasília, pelo relógio do servidor.
    pub today: Period,
    /// Dias em que a sala jogou, do mais recente para o mais antigo.
    pub played: Vec<Period>,
    /// Viradas da sala. Vazio enquanto houver menos de
    /// `rules::MIN_BOIS_FOR_COMEBACKS` bois terminados.
    pub comebacks: Vec<Comeback>,
    /// Cores das duplas já formadas nesta sala.
    pub pairs: Vec<PairColors>,
}

impl RoomView {
    pub fn player(&self, id: i64) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    pub fn player_name(&self, id: i64) -> String {
        self.player(id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "?".into())
    }

    /// Cor de cada lado de um boi: a congelada, ou a atual em bois antigos.
    pub fn side_colors(&self, boi: &BoiView) -> Vec<u8> {
        if boi.colors.len() == rules::side_count(self.mode, boi.seating.len()) {
            boi.colors.clone()
        } else {
            resolve_side_colors(self.mode, &boi.seating, &self.players, &self.pairs)
        }
    }

    pub fn current(&self) -> Option<&BoiView> {
        self.current.as_ref()
    }

    pub fn state(&self, boi: &BoiView) -> BoiState {
        rules::evaluate(self.mode, boi.seating.len(), &boi.hands)
    }

    /// Um boi está em andamento: já tem mão registrada e ainda não acabou.
    pub fn boi_in_progress(&self) -> bool {
        self.current()
            .is_some_and(|b| !b.hands.is_empty() && !self.state(b).finished())
    }

    /// Nome de um lado: o jogador (individual) ou os dois parceiros (duplas),
    /// em ordem alfabética, para a dupla ter sempre o mesmo nome.
    pub fn side_name(&self, boi: &BoiView, side: usize) -> String {
        self.pair_name(&sides_of(self.mode, &boi.seating)[side])
    }

    pub fn pair_name(&self, ids: &[i64]) -> String {
        let mut names: Vec<String> = ids.iter().map(|&id| self.player_name(id)).collect();
        names.sort_by_key(|n| sort_key(n));
        names.join(" & ")
    }

    /// Iniciais do lado, na mesma ordem do nome ("Ana & Carlos" → "AC").
    pub fn side_initials(&self, boi: &BoiView, side: usize) -> String {
        let mut names: Vec<String> = sides_of(self.mode, &boi.seating)[side]
            .iter()
            .map(|&id| self.player_name(id))
            .collect();
        names.sort_by_key(|n| sort_key(n));
        names
            .iter()
            .filter_map(|n| n.chars().next())
            .flat_map(char::to_uppercase)
            .collect()
    }
}

/// Números de um período. `firsts` só vem quando o período é um ano.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeriodReport {
    pub stats: Vec<PlayerStats>,
    /// Desempenho de cada dupla; vazio no modo individual.
    pub pairs: Vec<PairRecord>,
    pub firsts: Option<Firsts>,
}

/// Quantos dias o histórico traz por vez.
pub const HISTORY_DAYS: usize = 5;

/// Quantos bois um lado ganhou num dia. `players` em ordem crescente.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SideTally {
    pub players: Vec<i64>,
    pub won: u32,
}

/// Resumo de um dia de jogo: quantos bois terminaram e quantos cada lado
/// ganhou (do que ganhou mais para o que ganhou menos).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DaySummary {
    pub day: Period,
    pub bois: u32,
    pub sides: Vec<SideTally>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DaysPage {
    /// Do dia mais recente para o mais antigo.
    pub days: Vec<DaySummary>,
    /// Ainda há dias mais antigos.
    pub more: bool,
}

fn finished(mode: Mode, bois: &[BoiView]) -> impl DoubleEndedIterator<Item = &BoiView> {
    bois.iter()
        .filter(move |b| rules::evaluate(mode, b.seating.len(), &b.hands).finished())
}

/// Resumo de cada dia com boi terminado, do mais recente para o mais antigo.
/// `bois` vem do mais antigo para o mais recente.
pub fn day_summaries(mode: Mode, bois: &[BoiView]) -> Vec<DaySummary> {
    let mut days: Vec<DaySummary> = Vec::new();
    for boi in finished(mode, bois).rev() {
        let day = Period::day_of(boi.started_at);
        if days.last().is_none_or(|d| d.day != day) {
            days.push(DaySummary {
                day,
                bois: 0,
                sides: Vec::new(),
            });
        }
        let summary = days.last_mut().expect("acabou de entrar");
        summary.bois += 1;
        let winner = rules::evaluate(mode, boi.seating.len(), &boi.hands).winner;
        for (side, mut players) in sides_of(mode, &boi.seating).into_iter().enumerate() {
            players.sort();
            let won = u32::from(winner == Some(side));
            match summary.sides.iter_mut().find(|t| t.players == players) {
                Some(t) => t.won += won,
                None => summary.sides.push(SideTally { players, won }),
            }
        }
    }
    for d in &mut days {
        d.sides.sort_by_key(|t| std::cmp::Reverse(t.won));
    }
    days
}

/// Página do histórico: até `HISTORY_DAYS` dias anteriores a `before` (ou
/// os mais recentes, sem `before`).
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn days_page(mode: Mode, bois: &[BoiView], before: Option<Period>) -> DaysPage {
    let mut days = day_summaries(mode, bois)
        .into_iter()
        .filter(|d| before.is_none_or(|b| d.day.range().0 < b.range().0));
    let page: Vec<DaySummary> = days.by_ref().take(HISTORY_DAYS).collect();
    DaysPage {
        days: page,
        more: days.next().is_some(),
    }
}

/// Bois terminados de um dia, do mais recente para o mais antigo.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn day_bois(mode: Mode, bois: &[BoiView], day: Period) -> Vec<BoiView> {
    finished(mode, bois)
        .rev()
        .filter(|b| day.contains(b.started_at))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Hand, Outcome};

    const RODA: [i64; 4] = [1, 2, 3, 4];

    /// Boi de duplas que começou `days_ago` dias antes de 13/10/2026 e foi
    /// ganho pelo lado `winner` por 4 a 0 (ou que só teve uma mão).
    fn boi(id: i64, days_ago: i64, winner: usize, finished: bool) -> BoiView {
        let win = Hand {
            starter_seat: 0,
            outcome: Outcome::Batida,
            winner_side: Some(winner),
        };
        BoiView {
            id,
            number: id as u32,
            seating: RODA.to_vec(),
            hands: vec![win; if finished { 4 } else { 1 }],
            colors: Vec::new(),
            // 13/10/2026 12:00 em Brasília.
            started_at: 1_791_903_600 - days_ago * 86_400,
        }
    }

    #[test]
    fn historico_por_dia() {
        // Dia 13: três bois (A, A, B) e um em andamento. Dias 12 a 6: um boi por dia.
        let mut bois: Vec<BoiView> = (0..7).map(|i| boi(i + 1, 7 - i, 0, true)).collect();
        bois.push(boi(8, 0, 0, true));
        bois.push(boi(9, 0, 1, true));
        bois.push(boi(10, 0, 0, true));
        bois.push(boi(11, 0, 1, false));

        let first = days_page(Mode::Duplas, &bois, None);
        let labels = |p: &DaysPage| p.days.iter().map(|d| d.day.label()).collect::<Vec<_>>();
        assert_eq!(
            labels(&first),
            [
                "13/10/2026",
                "12/10/2026",
                "11/10/2026",
                "10/10/2026",
                "09/10/2026"
            ]
        );
        assert!(first.more);
        let hoje = &first.days[0];
        assert_eq!(hoje.bois, 3);
        // Dupla A (1 e 3) ganhou 2, dupla B (2 e 4) ganhou 1.
        assert_eq!(
            hoje.sides,
            vec![
                SideTally {
                    players: vec![1, 3],
                    won: 2
                },
                SideTally {
                    players: vec![2, 4],
                    won: 1
                },
            ]
        );
        let rest = days_page(Mode::Duplas, &bois, Some(first.days[4].day));
        assert_eq!(labels(&rest), ["08/10/2026", "07/10/2026", "06/10/2026"]);
        assert!(!rest.more);

        let ids: Vec<i64> = day_bois(Mode::Duplas, &bois, hoje.day)
            .iter()
            .map(|b| b.id)
            .collect();
        assert_eq!(ids, [10, 9, 8]);
    }
}
