//! Paleta de cores dos jogadores e das duplas.
//!
//! As cores são guardadas no banco pelo índice nesta lista. Para crescer para
//! 12 ou 16 cores, basta acrescentar no fim: os índices antigos não mudam.
//!
//! Valores calculados em `tools/cores.html` (casca OKLCH L=0,50 no tema claro e
//! 0,74 no escuro; miolo com 25% da cor sobre o fundo do cartão). Dourado,
//! vermelho e o cinza-azulado do empate ficam de fora: têm significado próprio.

pub struct PaletteColor {
    pub name: &'static str,
    /// Cor cheia (batida) e miolo (fechado) no tema claro.
    pub light: &'static str,
    pub light_core: &'static str,
    /// O mesmo no tema escuro.
    pub dark: &'static str,
    pub dark_core: &'static str,
}

pub const PALETTE: [PaletteColor; 8] = [
    PaletteColor { name: "Azul", light: "#1649ae", light_core: "#c2d0e9", dark: "#6095f9", dark_core: "#2f394b" },
    PaletteColor { name: "Ciano", light: "#1a8392", light_core: "#cbdede", dark: "#2fd4ec", dark_core: "#2e4648" },
    PaletteColor { name: "Turquesa", light: "#005b4d", light_core: "#c3d2ca", dark: "#0baa92", dark_core: "#283d35" },
    PaletteColor { name: "Verde", light: "#097f23", light_core: "#c8ddc2", dark: "#68cb6e", dark_core: "#33442e" },
    PaletteColor { name: "Lima", light: "#789301", light_core: "#dbe3c4", dark: "#c1e069", dark_core: "#43482e" },
    PaletteColor { name: "Roxo", light: "#7241a0", light_core: "#dacde4", dark: "#bb8aef", dark_core: "#413749" },
    PaletteColor { name: "Magenta", light: "#a12d79", light_core: "#ebcad7", dark: "#f47ac3", dark_core: "#4e343f" },
    PaletteColor { name: "Marrom", light: "#5f371b", light_core: "#d6c8bc", dark: "#a87b5d", dark_core: "#3e332a" },
];

pub fn is_valid(color: u8) -> bool {
    (color as usize) < PALETTE.len()
}

/// Primeira cor da paleta que não está em `taken`.
pub fn first_free(taken: &[u8]) -> u8 {
    (0..PALETTE.len() as u8)
        .find(|c| !taken.contains(c))
        .unwrap_or(0)
}

/// Variáveis CSS da cor, para o atributo `style`. O CSS escolhe entre as
/// versões clara e escura (`--c` cheia, `--k` miolo).
pub fn style(color: u8) -> String {
    let c = &PALETTE[(color as usize).min(PALETTE.len() - 1)];
    format!(
        "--cl:{};--kl:{};--cd:{};--kd:{}",
        c.light, c.light_core, c.dark, c.dark_core
    )
}
