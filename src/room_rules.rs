//! Regras da sala, fora do jogo em si: nome, jogadores, cores, começar boi,
//! marcar e desfazer mão. Sem I/O: o handler carrega os dados, pergunta aqui
//! e grava o que foi decidido. As regras do jogo ficam em `rules.rs`.

use crate::model::{BoiView, Mode, Outcome, Player, pair_key};
use crate::views::PlayerInput;
use crate::{palette, rules};

/// Por que um pedido foi recusado. O handler transforma em resposta HTTP.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Pedido mal formado (400).
    Invalid(String),
    /// O estado da sala não permite agora (409).
    Conflict(String),
    /// A pessoa não tem permissão (403).
    Forbidden(String),
}

fn invalid(msg: impl Into<String>) -> Refusal {
    Refusal::Invalid(msg.into())
}

fn conflict(msg: impl Into<String>) -> Refusal {
    Refusal::Conflict(msg.into())
}

const STALE: &str = "O placar mudou enquanto você marcava. Confira e tente de novo.";

/// Nome da sala sem espaços nas pontas; não pode ficar vazio.
pub fn room_name(name: &str) -> Result<String, Refusal> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid("Dê um nome para a sala."));
    }
    Ok(name.to_string())
}

/// Quantidade de jogadores certa para o modo, e todos com nome.
pub fn validate_players(mode: Mode, players: &[PlayerInput]) -> Result<(), Refusal> {
    if !rules::valid_player_count(mode, players.len()) {
        return Err(invalid(match mode {
            Mode::Individual => "No individual, a sala tem de 2 a 4 jogadores.",
            Mode::Duplas => "Nas duplas, a sala tem 4 jogadores.",
        }));
    }
    if players.iter().any(|p| p.name.trim().is_empty()) {
        return Err(invalid("Todo jogador precisa de um nome."));
    }
    Ok(())
}

/// Uma conta só pode ocupar um lugar na sala.
pub fn distinct_links(links: &[Option<i64>]) -> Result<(), Refusal> {
    let ids: Vec<i64> = links.iter().flatten().copied().collect();
    if (1..ids.len()).any(|i| ids[..i].contains(&ids[i])) {
        return Err(invalid("A mesma conta não pode ocupar dois lugares."));
    }
    Ok(())
}

/// Cor de cada jogador: a escolhida, ou a primeira livre. No individual, todos
/// jogam juntos, então as cores precisam ser diferentes.
pub fn assign_colors(players: &[PlayerInput]) -> Result<Vec<u8>, Refusal> {
    let mut colors: Vec<Option<u8>> = players.iter().map(|p| p.color).collect();
    if colors.iter().flatten().any(|&c| !palette::is_valid(c)) {
        return Err(invalid("Cor inválida."));
    }
    let chosen: Vec<u8> = colors.iter().flatten().copied().collect();
    if (1..chosen.len()).any(|i| chosen[..i].contains(&chosen[i])) {
        return Err(invalid("Dois jogadores estão com a mesma cor."));
    }
    for i in 0..colors.len() {
        if colors[i].is_none() {
            let taken: Vec<u8> = colors.iter().flatten().copied().collect();
            colors[i] = Some(palette::first_free(&taken));
        }
    }
    Ok(colors.into_iter().flatten().collect())
}

/// Boi novo só depois que o atual acabou.
pub fn can_start_boi(mode: Mode, current: Option<&BoiView>) -> Result<(), Refusal> {
    match current {
        Some(boi) if !rules::evaluate(mode, boi.seating.len(), &boi.hands).finished() => {
            Err(conflict("O boi atual ainda não acabou."))
        }
        _ => Ok(()),
    }
}

/// Onde gravar uma mão nova: o boi atual e a posição dela no boi.
/// `expected_hands` evita gravar duas vezes quando duas pessoas marcam ao
/// mesmo tempo ou o botão é tocado duas vezes.
pub fn place_hand(
    mode: Mode,
    current: Option<&BoiView>,
    expected_hands: usize,
    starter_seat: usize,
    outcome: Outcome,
    winner_side: Option<usize>,
) -> Result<(i64, usize), Refusal> {
    let boi = current.ok_or_else(|| conflict("Comece um boi primeiro."))?;
    if boi.hands.len() != expected_hands {
        return Err(conflict(STALE));
    }
    rules::validate_hand(
        mode,
        boi.seating.len(),
        &boi.hands,
        starter_seat,
        outcome,
        winner_side,
    )
    .map_err(invalid)?;
    Ok((boi.id, boi.hands.len()))
}

/// O que o desfazer apaga.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Undo {
    /// A última mão do boi atual (`seq` é a posição dela).
    Hand { boi_id: i64, seq: usize },
    /// O boi atual, que ainda não tem mãos: a sala volta ao boi anterior.
    Boi { boi_id: i64 },
}

pub fn undo(current: Option<&BoiView>, expected_hands: usize) -> Result<Undo, Refusal> {
    let boi = current.ok_or_else(|| conflict("Não há nada para desfazer."))?;
    if boi.hands.len() != expected_hands {
        return Err(conflict(STALE));
    }
    Ok(match boi.hands.len() {
        0 => Undo::Boi { boi_id: boi.id },
        n => Undo::Hand {
            boi_id: boi.id,
            seq: n - 1,
        },
    })
}

/// Confere a edição de jogadores e devolve quem sai da sala. Só o dono edita,
/// só fora de um boi em andamento, e quem já jogou algum boi não pode sair,
/// para não perder o histórico.
pub fn players_to_remove(
    is_owner: bool,
    boi_in_progress: bool,
    current: &[Player],
    edited: &[PlayerInput],
    bois: &[BoiView],
) -> Result<Vec<i64>, Refusal> {
    if !is_owner {
        return Err(Refusal::Forbidden(
            "Só o dono da sala pode editar os jogadores.".into(),
        ));
    }
    if boi_in_progress {
        return Err(conflict(
            "Não dá para mudar os jogadores no meio de um boi.",
        ));
    }
    let kept: Vec<i64> = edited.iter().filter_map(|p| p.id).collect();
    if kept.iter().any(|id| !current.iter().any(|p| p.id == *id)) {
        return Err(invalid("Jogador desconhecido."));
    }
    let removed: Vec<&Player> = current.iter().filter(|p| !kept.contains(&p.id)).collect();
    if let Some(p) = removed
        .iter()
        .find(|p| bois.iter().any(|b| b.seating.contains(&p.id)))
    {
        return Err(conflict(format!(
            "{} já jogou nesta sala e não pode ser removido.",
            p.name
        )));
    }
    Ok(removed.iter().map(|p| p.id).collect())
}

/// Confere a troca de cores de uma dupla e devolve a dupla em ordem.
pub fn pair_colors(
    mode: Mode,
    players: &[Player],
    pair: (i64, i64),
    primary: u8,
    secondary: u8,
) -> Result<(i64, i64), Refusal> {
    if mode != Mode::Duplas {
        return Err(invalid("Esta sala não é de duplas."));
    }
    let key = pair_key(pair.0, pair.1);
    let known = |id: i64| players.iter().any(|p| p.id == id);
    if key.0 == key.1 || !known(key.0) || !known(key.1) {
        return Err(invalid("Dupla desconhecida."));
    }
    if !palette::is_valid(primary) || !palette::is_valid(secondary) {
        return Err(invalid("Cor inválida."));
    }
    if primary == secondary {
        return Err(invalid("A cor reserva precisa ser diferente da principal."));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Hand;

    fn input(id: Option<i64>, name: &str, color: Option<u8>) -> PlayerInput {
        PlayerInput {
            id,
            name: name.into(),
            username: None,
            color,
        }
    }

    fn player(id: i64, name: &str) -> Player {
        Player {
            id,
            name: name.into(),
            username: None,
            color: 0,
        }
    }

    fn boi(id: i64, seating: Vec<i64>, wins: usize) -> BoiView {
        let win = Hand {
            starter_seat: 0,
            outcome: Outcome::Batida,
            winner_side: Some(0),
        };
        BoiView {
            id,
            number: 1,
            seating,
            hands: vec![win; wins],
            colors: Vec::new(),
            started_at: 0,
        }
    }

    #[test]
    fn nome_e_jogadores() {
        assert_eq!(room_name("  Casa  "), Ok("Casa".into()));
        assert!(room_name("   ").is_err());
        let dois = [input(None, "Ana", None), input(None, "Bia", None)];
        assert!(validate_players(Mode::Individual, &dois).is_ok());
        assert!(validate_players(Mode::Duplas, &dois).is_err());
        let sem_nome = [input(None, "Ana", None), input(None, " ", None)];
        assert!(validate_players(Mode::Individual, &sem_nome).is_err());
        assert!(distinct_links(&[Some(1), None, None, Some(2)]).is_ok());
        assert!(distinct_links(&[Some(1), None, Some(1)]).is_err());
    }

    #[test]
    fn cores_escolhidas_e_livres() {
        let ps = [
            input(None, "A", Some(2)),
            input(None, "B", None),
            input(None, "C", Some(0)),
        ];
        assert_eq!(assign_colors(&ps), Ok(vec![2, 1, 0]));
        let repetida = [input(None, "A", Some(3)), input(None, "B", Some(3))];
        assert!(assign_colors(&repetida).is_err());
        assert!(assign_colors(&[input(None, "A", Some(200))]).is_err());
    }

    #[test]
    fn boi_novo_marcar_e_desfazer() {
        let jogando = boi(7, vec![1, 2], 2);
        let acabou = boi(7, vec![1, 2], 4);
        assert!(can_start_boi(Mode::Individual, None).is_ok());
        assert!(can_start_boi(Mode::Individual, Some(&acabou)).is_ok());
        assert!(matches!(
            can_start_boi(Mode::Individual, Some(&jogando)),
            Err(Refusal::Conflict(_))
        ));

        let marca = |cur, expected| {
            place_hand(Mode::Individual, cur, expected, 0, Outcome::Batida, Some(1))
        };
        assert_eq!(marca(Some(&jogando), 2), Ok((7, 2)));
        assert_eq!(marca(Some(&jogando), 1), Err(conflict(STALE)));
        assert!(matches!(marca(None, 0), Err(Refusal::Conflict(_))));
        assert!(matches!(marca(Some(&acabou), 4), Err(Refusal::Invalid(_))));

        assert_eq!(
            undo(Some(&jogando), 2),
            Ok(Undo::Hand { boi_id: 7, seq: 1 })
        );
        assert_eq!(
            undo(Some(&boi(8, vec![1, 2], 0)), 0),
            Ok(Undo::Boi { boi_id: 8 })
        );
        assert_eq!(undo(Some(&jogando), 3), Err(conflict(STALE)));
        assert!(undo(None, 0).is_err());
    }

    #[test]
    fn quem_ja_jogou_nao_sai() {
        let sala = [player(1, "Ana"), player(2, "Bia"), player(3, "Caio")];
        let bois = [boi(1, vec![1, 2], 4)];
        let sem_caio = [input(Some(1), "Ana", None), input(Some(2), "Bia", None)];
        assert_eq!(
            players_to_remove(true, false, &sala, &sem_caio, &bois),
            Ok(vec![3])
        );
        let sem_bia = [input(Some(1), "Ana", None), input(Some(3), "Caio", None)];
        assert_eq!(
            players_to_remove(true, false, &sala, &sem_bia, &bois),
            Err(conflict("Bia já jogou nesta sala e não pode ser removido."))
        );
        assert!(matches!(
            players_to_remove(false, false, &sala, &sem_caio, &bois),
            Err(Refusal::Forbidden(_))
        ));
        assert!(players_to_remove(true, true, &sala, &sem_caio, &bois).is_err());
        let estranho = [input(Some(9), "X", None), input(Some(1), "Ana", None)];
        assert_eq!(
            players_to_remove(true, false, &sala, &estranho, &bois),
            Err(invalid("Jogador desconhecido."))
        );
    }

    #[test]
    fn cores_da_dupla() {
        let sala = [
            player(1, "A"),
            player(2, "B"),
            player(3, "C"),
            player(4, "D"),
        ];
        assert_eq!(pair_colors(Mode::Duplas, &sala, (3, 1), 0, 1), Ok((1, 3)));
        assert!(pair_colors(Mode::Individual, &sala, (1, 3), 0, 1).is_err());
        assert!(pair_colors(Mode::Duplas, &sala, (1, 1), 0, 1).is_err());
        assert!(pair_colors(Mode::Duplas, &sala, (1, 9), 0, 1).is_err());
        assert!(pair_colors(Mode::Duplas, &sala, (1, 3), 2, 2).is_err());
    }
}
