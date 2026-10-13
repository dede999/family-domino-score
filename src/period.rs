//! Datas e períodos no fuso de Brasília.

use serde::{Deserialize, Serialize};

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

    #[test]
    fn data_em_brasilia() {
        // 2026-10-12 02:00 UTC ainda é dia 11 em Brasília.
        assert_eq!(super::format_date(1_791_770_400), "11/10/2026");
        assert_eq!(super::format_date(0), "31/12/1969");
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
