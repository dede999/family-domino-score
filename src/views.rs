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
    /// Primeira página do histórico: bois terminados, do mais recente para o
    /// mais antigo. As outras vêm de `api::room_history`.
    pub history: HistoryPage,
    /// Estatísticas de todos os bois da sala, calculadas no servidor.
    pub stats: Vec<PlayerStats>,
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

/// Quantos bois o histórico mostra por vez.
pub const HISTORY_PAGE: usize = 10;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoryPage {
    /// Do mais recente para o mais antigo.
    pub bois: Vec<BoiView>,
    /// Ainda há bois mais antigos que os desta página.
    pub more: bool,
}

/// Página do histórico: até `HISTORY_PAGE` bois terminados com id menor que
/// `before` (ou os mais recentes, sem `before`). `bois` vem do mais antigo
/// para o mais recente.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn history_page(mode: Mode, bois: &[BoiView], before: Option<i64>) -> HistoryPage {
    let mut finished = bois
        .iter()
        .rev()
        .filter(|b| before.is_none_or(|id| b.id < id))
        .filter(|b| rules::evaluate(mode, b.seating.len(), &b.hands).finished());
    let page: Vec<BoiView> = finished.by_ref().take(HISTORY_PAGE).cloned().collect();
    HistoryPage {
        bois: page,
        more: finished.next().is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Hand, Outcome};

    const RODA: [i64; 4] = [1, 2, 3, 4];

    /// Boi de duplas: terminado (4 batidas do lado 0) ou com uma mão só.
    fn boi(id: i64, finished: bool) -> BoiView {
        let win = Hand {
            starter_seat: 0,
            outcome: Outcome::Batida,
            winner_side: Some(0),
        };
        BoiView {
            id,
            number: id as u32,
            seating: RODA.to_vec(),
            hands: vec![win; if finished { 4 } else { 1 }],
            colors: Vec::new(),
            started_at: 0,
        }
    }

    #[test]
    fn historico_em_paginas_de_dez() {
        // 23 bois terminados e o atual em andamento.
        let mut bois: Vec<BoiView> = (1..=23).map(|id| boi(id, true)).collect();
        bois.push(boi(24, false));
        let ids = |p: &HistoryPage| p.bois.iter().map(|b| b.id).collect::<Vec<_>>();

        let first = history_page(Mode::Duplas, &bois, None);
        assert_eq!(ids(&first), (14..=23).rev().collect::<Vec<_>>());
        assert!(first.more);
        let second = history_page(Mode::Duplas, &bois, Some(14));
        assert_eq!(ids(&second), (4..=13).rev().collect::<Vec<_>>());
        assert!(second.more);
        let last = history_page(Mode::Duplas, &bois, Some(4));
        assert_eq!(ids(&last), vec![3, 2, 1]);
        assert!(!last.more);
    }
}
