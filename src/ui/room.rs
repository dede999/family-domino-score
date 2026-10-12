use super::new_room::{PlayerDraft, PlayersEditor, seat_label};
use super::{Cross, current_user, sleep_ms, use_me_resource};
use crate::Route;
use crate::api::{self, error_message};
use crate::model::{BoiView, Mode, Outcome, RoomView, format_date};
use crate::rules::{self, SideResult};
use dioxus::prelude::*;

#[component]
pub fn Room(code: String) -> Element {
    let code = use_signal(|| code.trim().to_uppercase());
    let mut room = use_resource(move || async move { api::get_room(code()).await });

    // Tempo real: o servidor avisa quando o placar muda, e a sala recarrega.
    use_effect(move || {
        spawn(async move {
            loop {
                if let Ok(mut events) = api::room_events(code()).await {
                    while let Some(Ok(_)) = events.recv().await {
                        room.restart();
                    }
                }
                // Conexão caiu: espera um pouco, recarrega e reconecta.
                sleep_ms(3000).await;
                room.restart();
            }
        });
    });

    let body = match &*room.read() {
        None => rsx! { p { class: "muted", "Carregando…" } },
        Some(Err(e)) => rsx! {
            p { class: "error", "{error_message(e)}" }
        },
        Some(Ok(view)) => rsx! {
            RoomBody { view: view.clone(), on_change: move |_| room.restart() }
        },
    };

    rsx! {
        main { class: "page",
            header { class: "top",
                Link { to: Route::Home {}, "← Início" }
            }
            {body}
        }
    }
}

#[component]
fn RoomBody(view: RoomView, on_change: EventHandler<()>) -> Element {
    let me = use_me_resource();
    let mut editing = use_signal(|| false);
    let logged_in = current_user(me).is_some();

    let mode_label = match view.mode {
        Mode::Individual => "Individual",
        Mode::Duplas => "Duplas",
    };

    rsx! {
        section { class: "room-head",
            h1 { "{view.name}" }
            p { class: "muted",
                "Código "
                strong { class: "code", "{view.code}" }
                " · {mode_label} · sala de {view.owner}"
            }
        }

        match view.current() {
            None => rsx! {
                section { class: "card",
                    p { "Nenhum boi ainda." }
                    if view.can_edit {
                        StartBoiButton { code: view.code.clone(), on_change }
                    }
                }
            },
            Some(boi) => rsx! {
                Scoreboard { view: view.clone(), boi: boi.clone(), on_change }
            },
        }

        if !view.can_edit {
            p { class: "muted center",
                if logged_in {
                    "Você está assistindo. Só o dono da sala e os jogadores com conta marcam o placar."
                } else {
                    "Você está assistindo. O placar atualiza sozinho."
                }
            }
        }

        if view.is_owner {
            section { class: "card",
                div { class: "row spread",
                    h2 { "Jogadores" }
                    if !editing() {
                        button {
                            class: "link",
                            disabled: view.boi_in_progress(),
                            title: if view.boi_in_progress() { "Termine o boi para mudar os jogadores" },
                            onclick: move |_| editing.set(true),
                            "Editar"
                        }
                    }
                }
                if editing() {
                    EditPlayers {
                        view: view.clone(),
                        on_done: move |changed: bool| {
                            editing.set(false);
                            if changed {
                                on_change.call(());
                            }
                        },
                    }
                } else {
                    ol { class: "seating",
                        for (seat, p) in view.players.iter().enumerate() {
                            li { key: "{p.id}",
                                span { class: "seat", {seat_label(view.mode, seat)} }
                                "{p.name}"
                                if let Some(u) = &p.username {
                                    span { class: "muted", " @{u}" }
                                } else {
                                    span { class: "muted", " (avulso)" }
                                }
                            }
                        }
                    }
                    if view.boi_in_progress() {
                        p { class: "muted", "A ordem só muda entre um boi e outro." }
                    }
                }
            }
        }

        Stats { view: view.clone() }
        History { view: view.clone() }
    }
}

#[component]
fn StartBoiButton(code: String, on_change: EventHandler<()>) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        button {
            class: "primary",
            disabled: busy(),
            onclick: move |_| {
                let code = code.clone();
                async move {
                    busy.set(true);
                    error.set(None);
                    match api::start_boi(code).await {
                        Ok(()) => on_change.call(()),
                        Err(e) => error.set(Some(error_message(&e))),
                    }
                    busy.set(false);
                }
            },
            "Começar boi"
        }
        if let Some(err) = error() {
            p { class: "error", "{err}" }
        }
    }
}

#[component]
fn Scoreboard(view: RoomView, boi: BoiView, on_change: EventHandler<()>) -> Element {
    let state = view.state(&boi);
    let players = boi.seating.len();
    let sides = rules::side_count(view.mode, players);
    let hands_len = boi.hands.len();

    // Quem saiu na mão que está sendo jogada. Volta para a sugestão sempre que
    // o número de mãos muda (mão registrada, desfeita, ou outra pessoa marcou).
    let mut starter = use_signal(|| state.next_starter);
    let mut choosing = use_signal(|| false);
    let suggested = state.next_starter;
    use_effect(use_reactive!(|hands_len, suggested| {
        let _ = hands_len;
        starter.set(suggested);
        choosing.set(false);
    }));

    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let code = view.code.clone();
    let record = use_callback(move |(outcome, winner): (Outcome, Option<usize>)| {
        let Some(starter_seat) = starter() else {
            error.set(Some("Escolha antes quem saiu.".into()));
            return;
        };
        let code = code.clone();
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = api::record_hand(code, hands_len, starter_seat, outcome, winner).await;
            busy.set(false);
            match result {
                Ok(()) => on_change.call(()),
                Err(e) => {
                    error.set(Some(error_message(&e)));
                    on_change.call(());
                }
            }
        });
    });

    let code = view.code.clone();
    let undo = move |_| {
        let code = code.clone();
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = api::undo(code, hands_len).await;
            busy.set(false);
            if let Err(e) = result {
                error.set(Some(error_message(&e)));
            }
            on_change.call(());
        });
    };

    let finished = state.finished();
    let ask_starter = !finished && (starter().is_none() || choosing());

    rsx! {
        section { class: "card board",
            h2 { "Boi nº {boi.number}" }
            div { class: if sides > 2 { "crosses many" } else { "crosses" },
                for side in 0..sides {
                    div { key: "{side}", class: "side",
                        Cross { score: state.scores[side], winner: state.winner == Some(side) }
                        div { class: "side-name", "{view.side_name(&boi, side)}" }
                        div { class: "score", "{state.scores[side]}" }
                    }
                }
            }

            if finished {
                FinalResult { view: view.clone(), boi: boi.clone() }
                if view.can_edit {
                    div { class: "actions",
                        StartBoiButton { code: view.code.clone(), on_change }
                    }
                }
            } else if view.can_edit {
                div { class: "starter",
                    if ask_starter {
                        p {
                            if suggested.is_none() {
                                strong { "Quem saiu? " }
                                span { class: "muted", "(maior carroça, começando pela sena)" }
                            } else {
                                strong { "Quem saiu?" }
                            }
                        }
                        div { class: "chips",
                            for (seat, &id) in boi.seating.iter().enumerate() {
                                button {
                                    key: "{seat}",
                                    class: if starter() == Some(seat) { "chip on" } else { "chip" },
                                    onclick: move |_| {
                                        starter.set(Some(seat));
                                        choosing.set(false);
                                    },
                                    "{view.player_name(id)}"
                                }
                            }
                        }
                    } else if let Some(seat) = starter() {
                        p {
                            "Sai: "
                            strong { "{view.player_name(boi.seating[seat])}" }
                            button { class: "link", onclick: move |_| choosing.set(true), " trocar" }
                        }
                    }
                }

                div { class: "win-buttons",
                    for side in 0..sides {
                        div { key: "{side}", class: "win-group",
                            span { class: "win-name", "{view.side_name(&boi, side)}" }
                            button {
                                disabled: busy() || starter().is_none(),
                                onclick: move |_| record.call((Outcome::Batida, Some(side))),
                                "Bateu"
                            }
                            button {
                                disabled: busy() || starter().is_none(),
                                onclick: move |_| record.call((Outcome::Fechado, Some(side))),
                                "Ganhou no fechado"
                            }
                        }
                    }
                    button {
                        class: "secondary",
                        disabled: busy() || starter().is_none(),
                        onclick: move |_| record.call((Outcome::Empate, None)),
                        "Fechado empatado"
                    }
                }
            }

            if let Some(err) = error() {
                p { class: "error", "{err}" }
            }

            if view.can_edit {
                div { class: "actions",
                    button {
                        class: "link",
                        disabled: busy(),
                        onclick: undo,
                        if boi.hands.is_empty() { "Desfazer: apagar este boi" } else { "Desfazer última mão" }
                    }
                }
            }

            if !boi.hands.is_empty() {
                HandList { view: view.clone(), boi: boi.clone() }
            }
        }
    }
}

#[component]
fn FinalResult(view: RoomView, boi: BoiView) -> Element {
    let state = view.state(&boi);
    let Some(winner) = state.winner else {
        return rsx! {};
    };
    let mut lambretas = Vec::new();
    let mut reversas = Vec::new();
    for (side, result) in state.results.iter().enumerate() {
        match result {
            Some(SideResult::Lambreta) => lambretas.push(view.side_name(&boi, side)),
            Some(SideResult::LambretaReversa) => reversas.push(view.side_name(&boi, side)),
            _ => {}
        }
    }
    rsx! {
        div { class: "result",
            p { class: "big", "{view.side_name(&boi, winner)} ganhou o boi!" }
            if !lambretas.is_empty() {
                p { class: "lambreta", "Lambreta em {lambretas.join(\", \")}!" }
            }
            if !reversas.is_empty() {
                p { class: "lambreta", "Lambreta reversa em {reversas.join(\", \")}!" }
            }
        }
    }
}

#[component]
fn HandList(view: RoomView, boi: BoiView) -> Element {
    rsx! {
        details { class: "hands",
            summary { "Mãos deste boi ({boi.hands.len()})" }
            ol { reversed: true,
                for (i, hand) in boi.hands.iter().enumerate().rev() {
                    li { key: "{i}",
                        {
                            let starter = view.player_name(boi.seating[hand.starter_seat]);
                            match (hand.outcome, hand.winner_side) {
                                (Outcome::Empate, _) | (_, None) => {
                                    format!("{starter} saiu · fechado empatado")
                                }
                                (Outcome::Batida, Some(s)) => {
                                    format!("{starter} saiu · {} bateu", view.side_name(&boi, s))
                                }
                                (Outcome::Fechado, Some(s)) => {
                                    format!("{starter} saiu · {} ganhou no fechado", view.side_name(&boi, s))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EditPlayers(view: RoomView, on_done: EventHandler<bool>) -> Element {
    let initial: Vec<PlayerDraft> = view
        .players
        .iter()
        .map(|p| PlayerDraft {
            id: Some(p.id),
            name: p.name.clone(),
            username: p.username.clone().unwrap_or_default(),
        })
        .collect();
    let players = use_signal(|| initial);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let code = view.code.clone();

    rsx! {
        PlayersEditor { mode: view.mode, players }
        if let Some(err) = error() {
            p { class: "error", "{err}" }
        }
        div { class: "row",
            button {
                disabled: busy(),
                onclick: move |_| {
                    let code = code.clone();
                    let list = players().iter().map(PlayerDraft::to_input).collect();
                    async move {
                        busy.set(true);
                        error.set(None);
                        match api::update_players(code, list).await {
                            Ok(()) => on_done.call(true),
                            Err(e) => error.set(Some(error_message(&e))),
                        }
                        busy.set(false);
                    }
                },
                "Salvar"
            }
            button { class: "link", onclick: move |_| on_done.call(false), "Cancelar" }
        }
    }
}

#[component]
fn Stats(view: RoomView) -> Element {
    let mut stats = view.stats();
    if stats.is_empty() {
        return rsx! {};
    }
    stats.sort_by(|a, b| {
        b.bois_ganhos
            .cmp(&a.bois_ganhos)
            .then(b.maos_ganhas.cmp(&a.maos_ganhas))
    });
    rsx! {
        section { class: "card",
            h2 { "Estatísticas da sala" }
            div { class: "table-wrap",
                table { class: "stats-table",
                    thead {
                        tr {
                            th { "Jogador" }
                            th { title: "Bois ganhos / jogados", "Bois" }
                            th { title: "Mãos ganhas / jogadas", "Mãos" }
                            th { title: "Batidas", "Bat." }
                            th { title: "Ganhou no fechado", "Fech." }
                            th { title: "Lambretas aplicadas / sofridas", "Lamb." }
                            th { title: "Lambretas reversas aplicadas / sofridas", "Rev." }
                        }
                    }
                    tbody {
                        for s in stats.iter() {
                            tr { key: "{s.player_id}",
                                td { "{view.player_name(s.player_id)}" }
                                td { "{s.bois_ganhos}/{s.bois_jogados}" }
                                td { "{s.maos_ganhas}/{s.maos_jogadas}" }
                                td { "{s.batidas}" }
                                td { "{s.fechados}" }
                                td { "{s.lambretas_aplicadas}/{s.lambretas_sofridas}" }
                                td { "{s.reversas_aplicadas}/{s.reversas_sofridas}" }
                            }
                        }
                    }
                }
            }
            p { class: "muted small",
                "Lambretas e reversas: aplicadas/sofridas. Nas duplas, a vitória da dupla conta para os dois parceiros."
            }
        }
    }
}

/// Bois já terminados, agrupados por dia de jogo.
#[component]
fn History(view: RoomView) -> Element {
    let finished: Vec<&BoiView> = view
        .bois
        .iter()
        .filter(|b| view.state(b).finished())
        .rev()
        .collect();
    if finished.is_empty() {
        return rsx! {};
    }
    let mut days: Vec<(String, Vec<&BoiView>)> = Vec::new();
    for boi in finished {
        let day = format_date(boi.started_at);
        match days.last_mut() {
            Some((d, list)) if *d == day => list.push(boi),
            _ => days.push((day, vec![boi])),
        }
    }
    rsx! {
        section { class: "card",
            h2 { "Histórico" }
            for (day, bois) in days.iter() {
                div { key: "{day}", class: "day",
                    h3 { "{day}" }
                    ul {
                        for boi in bois.iter() {
                            li { key: "{boi.id}", {history_line(&view, boi)} }
                        }
                    }
                }
            }
        }
    }
}

fn history_line(view: &RoomView, boi: &BoiView) -> String {
    let state = view.state(boi);
    let Some(winner) = state.winner else {
        return String::new();
    };
    let others: Vec<String> = (0..state.scores.len())
        .filter(|&s| s != winner)
        .map(|s| {
            let tag = match state.results[s] {
                Some(SideResult::Lambreta) => " (lambreta)",
                Some(SideResult::LambretaReversa) => " (lambreta reversa)",
                _ => "",
            };
            format!("{} {}{tag}", view.side_name(boi, s), state.scores[s])
        })
        .collect();
    format!(
        "Boi {}: {} 4 × {}",
        boi.number,
        view.side_name(boi, winner),
        others.join(", ")
    )
}
