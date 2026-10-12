//! Tipos trocados entre o servidor e o navegador.

use crate::rules::{self, BoiState, PlayerStats};
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
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoomSummary {
    pub code: String,
    pub name: String,
    pub mode: Mode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub id: i64,
    pub name: String,
    /// Conta vinculada. `None` para jogador avulso.
    pub username: Option<String>,
}

/// Jogador na criação ou edição de uma sala.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    /// `None` para jogador novo.
    pub id: Option<i64>,
    pub name: String,
    /// Vincular a uma conta existente pelo nome de usuário.
    pub username: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoiView {
    pub id: i64,
    pub number: u32,
    /// Ids dos jogadores na ordem dos assentos, congelada no início do boi.
    pub seating: Vec<i64>,
    pub hands: Vec<Hand>,
    /// Início do boi em segundos desde 1970 (UTC).
    pub started_at: i64,
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
    /// Do mais antigo para o mais recente. O último é o boi atual.
    pub bois: Vec<BoiView>,
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

    pub fn current(&self) -> Option<&BoiView> {
        self.bois.last()
    }

    pub fn state(&self, boi: &BoiView) -> BoiState {
        rules::evaluate(self.mode, boi.seating.len(), &boi.hands)
    }

    /// Um boi está em andamento: já tem mão registrada e ainda não acabou.
    pub fn boi_in_progress(&self) -> bool {
        self.current()
            .is_some_and(|b| !b.hands.is_empty() && !self.state(b).finished())
    }

    /// Nome de um lado: o jogador (individual) ou os dois parceiros (duplas).
    pub fn side_name(&self, boi: &BoiView, side: usize) -> String {
        let names: Vec<String> = boi
            .seating
            .iter()
            .enumerate()
            .filter(|(seat, _)| rules::side_of(self.mode, *seat) == side)
            .map(|(_, &id)| self.player_name(id))
            .collect();
        names.join(" & ")
    }

    pub fn stats(&self) -> Vec<PlayerStats> {
        rules::compute_stats(
            self.bois
                .iter()
                .map(|b| (self.mode, b.seating.as_slice(), b.hands.as_slice())),
        )
    }
}

/// Data no fuso de Brasília (UTC-3, sem horário de verão), como "dd/mm/aaaa".
pub fn format_date(epoch_secs: i64) -> String {
    let days = (epoch_secs - 3 * 3600).div_euclid(86_400);
    // Algoritmo civil_from_days, de Howard Hinnant.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{d:02}/{m:02}/{y}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn data_em_brasilia() {
        // 2026-10-12 02:00 UTC ainda é dia 11 em Brasília.
        assert_eq!(super::format_date(1_791_770_400), "11/10/2026");
        assert_eq!(super::format_date(0), "31/12/1969");
    }
}
