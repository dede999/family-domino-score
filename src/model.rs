//! Tipos trocados entre o servidor e o navegador.

use crate::palette;
use crate::rules::{self, BoiState, Comeback, Firsts, PairRecord, PlayerStats};
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
    /// Índice na paleta. Só vale no modo individual; nas duplas a cor é da dupla.
    pub color: u8,
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

/// Fuso de Brasília: UTC-3, sem horário de verão.
const BRT: i64 = 3 * 3600;

/// Data no fuso de Brasília, como "dd/mm/aaaa".
pub fn format_date(epoch_secs: i64) -> String {
    Period::day_of(epoch_secs).label()
}

/// Dia (ano, mês, dia) a partir de dias desde 1970. Algoritmo
/// civil_from_days, de Howard Hinnant.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y as i32, m as u32, d as u32)
}

/// O inverso de `civil_from_days` (days_from_civil, também de Hinnant).
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = i64::from(y) - if m <= 2 { 1 } else { 0 };
    let (m, d) = (i64::from(m), i64::from(d));
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

const MONTHS: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

/// Período das estatísticas, no fuso de Brasília. Um boi entra no período
/// do dia em que começou.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Period {
    Day { year: i32, month: u32, day: u32 },
    Month { year: i32, month: u32 },
    Year { year: i32 },
}

impl Period {
    pub fn day_of(epoch_secs: i64) -> Period {
        let (year, month, day) = civil_from_days((epoch_secs - BRT).div_euclid(86_400));
        Period::Day { year, month, day }
    }

    /// O mês ou o ano que contém este período.
    pub fn month(self) -> Period {
        match self {
            Period::Day { year, month, .. } | Period::Month { year, month } => {
                Period::Month { year, month }
            }
            Period::Year { .. } => self,
        }
    }

    pub fn year(self) -> Period {
        match self {
            Period::Day { year, .. } | Period::Month { year, .. } | Period::Year { year } => {
                Period::Year { year }
            }
        }
    }

    /// Intervalo [início, fim) em segundos desde 1970 (UTC).
    pub fn range(self) -> (i64, i64) {
        let (start, end) = match self {
            Period::Day { year, month, day } => {
                let d = days_from_civil(year, month, day);
                (d, d + 1)
            }
            Period::Month { year, month } => {
                let next = if month == 12 {
                    (year + 1, 1)
                } else {
                    (year, month + 1)
                };
                (
                    days_from_civil(year, month, 1),
                    days_from_civil(next.0, next.1, 1),
                )
            }
            Period::Year { year } => (days_from_civil(year, 1, 1), days_from_civil(year + 1, 1, 1)),
        };
        (start * 86_400 + BRT, end * 86_400 + BRT)
    }

    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub fn contains(self, epoch_secs: i64) -> bool {
        let (start, end) = self.range();
        (start..end).contains(&epoch_secs)
    }

    /// "13/10/2026", "outubro de 2026" ou "2026".
    pub fn label(self) -> String {
        match self {
            Period::Day { year, month, day } => format!("{day:02}/{month:02}/{year}"),
            Period::Month { year, month } => format!("{} de {year}", MONTHS[month as usize - 1]),
            Period::Year { year } => year.to_string(),
        }
    }
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

    #[test]
    fn data_em_brasilia() {
        // 2026-10-12 02:00 UTC ainda é dia 11 em Brasília.
        assert_eq!(super::format_date(1_791_770_400), "11/10/2026");
        assert_eq!(super::format_date(0), "31/12/1969");
    }

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

    #[test]
    fn periodos_no_fuso_de_brasilia() {
        // 13/10/2026 00:00 em Brasília = 03:00 UTC.
        let dia = Period::Day {
            year: 2026,
            month: 10,
            day: 13,
        };
        assert_eq!(dia.range(), (1_791_860_400, 1_791_860_400 + 86_400));
        assert_eq!(Period::day_of(1_791_860_400), dia);
        assert_eq!(
            Period::day_of(1_791_860_399),
            Period::Day {
                year: 2026,
                month: 10,
                day: 12
            }
        );
        // Dezembro vira o ano; fevereiro de ano bissexto tem 29 dias.
        let (_, fim) = Period::Month {
            year: 2026,
            month: 12,
        }
        .range();
        assert_eq!(fim, Period::Year { year: 2027 }.range().0);
        let (ini, fim) = Period::Month {
            year: 2028,
            month: 2,
        }
        .range();
        assert_eq!((fim - ini) / 86_400, 29);
        assert!(Period::Year { year: 2026 }.contains(1_791_860_400));
        assert_eq!(dia.month().label(), "outubro de 2026");
        assert_eq!(dia.year().label(), "2026");
    }
}
