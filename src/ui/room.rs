use super::colors::{PairColorsCard, Swatch};
use super::new_room::{PlayerDraft, PlayersEditor, seat_label};
use super::{Cross, CrossEnd, current_user, sleep_ms, use_me_resource};
use crate::Route;
use crate::api::{self, error_message};
use crate::model::{BoiView, Mode, Outcome, Period, RoomView, format_date};
use crate::palette;
use crate::rules::{self, Firsts, Moment, PlayerStats, SideResult};
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
        section { class: "page-head",
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

        if view.can_edit && view.mode == Mode::Duplas {
            PairColorsCard { view: view.clone(), on_change }
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
                                if view.mode == Mode::Individual {
                                    Swatch { color: p.color }
                                    " "
                                }
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
    let colors = view.side_colors(&boi);

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
                        Cross {
                            wins: boi
                                .hands
                                .iter()
                                .filter(|h| h.winner_side == Some(side))
                                .map(|h| h.outcome)
                                .collect::<Vec<_>>(),
                            color: colors[side],
                            end: match (state.winner, state.results[side]) {
                                (None, _) => CrossEnd::Playing,
                                (Some(w), _) if w == side => CrossEnd::Winner,
                                (_, Some(SideResult::Lambreta)) => CrossEnd::Lambreta,
                                _ => CrossEnd::Loser,
                            },
                        }
                        div { class: "side-name", "{view.side_name(&boi, side)}" }
                        div { class: "score", "{state.scores[side]}" }
                    }
                }
            }

            if !boi.hands.is_empty() {
                HandStrip { view: view.clone(), boi: boi.clone(), colors: colors.clone() }
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
                        p { class: "starter-line",
                            span {
                                "Sai: "
                                strong { "{view.player_name(boi.seating[seat])}" }
                            }
                            button { class: "link", onclick: move |_| choosing.set(true), "trocar" }
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

/// Mãos do boi, na ordem em que aconteceram: quadrado cheio na cor de quem
/// bateu, casca com miolo claro no fechado, cinza com "=" no empate. Nas
/// duplas vira um retângulo com as iniciais da dupla que venceu. A última
/// fica destacada. Tocar num quadrado mostra o que aconteceu naquela mão.
#[component]
fn HandStrip(view: RoomView, boi: BoiView, colors: Vec<u8>) -> Element {
    let mut selected = use_signal(|| None::<usize>);
    let last = boi.hands.len() - 1;
    // Se a mão escolhida foi desfeita, esquece a escolha.
    let shown = selected().filter(|&i| i <= last);
    rsx! {
        div { class: "hand-strip", role: "group", "aria-label": "Mãos deste boi",
            for (i, hand) in boi.hands.iter().enumerate() {
                button {
                    key: "{i}",
                    r#type: "button",
                    class: {
                        let kind = match hand.outcome {
                            Outcome::Batida => "batida",
                            Outcome::Fechado => "fechado",
                            Outcome::Empate => "empate",
                        };
                        let mut c = format!("hand-square {kind}");
                        if view.mode == Mode::Duplas {
                            c.push_str(" pair");
                        }
                        if i == last {
                            c.push_str(" last");
                        }
                        if shown == Some(i) {
                            c.push_str(" on");
                        }
                        c
                    },
                    style: hand.winner_side.map(|s| palette::style(colors[s])),
                    title: hand_text(&view, &boi, i),
                    "aria-pressed": shown == Some(i),
                    onclick: move |_| selected.set(if shown == Some(i) { None } else { Some(i) }),
                    match (hand.outcome, hand.winner_side) {
                        (Outcome::Empate, _) | (_, None) => "=".to_string(),
                        (_, Some(s)) if view.mode == Mode::Duplas => view.side_initials(&boi, s),
                        _ => String::new(),
                    }
                }
            }
        }
        if let Some(i) = shown {
            p { class: "hand-detail muted small", {hand_text(&view, &boi, i)} }
        }
    }
}

fn hand_text(view: &RoomView, boi: &BoiView, i: usize) -> String {
    let hand = &boi.hands[i];
    let starter = view.player_name(boi.seating[hand.starter_seat]);
    let what = match (hand.outcome, hand.winner_side) {
        (Outcome::Empate, _) | (_, None) => "fechado empatado".to_string(),
        (Outcome::Batida, Some(s)) => format!("{} bateu", view.side_name(boi, s)),
        (Outcome::Fechado, Some(s)) => format!("{} ganhou no fechado", view.side_name(boi, s)),
    };
    format!("Mão {}: {starter} saiu, {what}", i + 1)
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
            color: Some(p.color),
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

#[derive(Clone, Copy, PartialEq)]
enum Scope {
    Tudo,
    Dia,
    Mes,
    Ano,
}

/// Estatísticas da sala: de tudo, de um dia, de um mês ou de um ano. Meses e
/// anos podem ser comparados dois a dois.
#[component]
fn Stats(view: RoomView) -> Element {
    let mut scope = use_signal(|| Scope::Tudo);
    let mut chosen = use_signal(|| None::<Period>);
    let mut other = use_signal(|| None::<Period>);

    let mut options: Vec<Period> = Vec::new();
    for day in &view.played {
        let p = match scope() {
            Scope::Tudo => continue,
            Scope::Dia => *day,
            Scope::Mes => day.month(),
            Scope::Ano => day.year(),
        };
        if !options.contains(&p) {
            options.push(p);
        }
    }
    // Sem escolha (ou escolha que sumiu), vale o período mais recente.
    let a = chosen()
        .filter(|p| options.contains(p))
        .or(options.first().copied());
    let b = other()
        .filter(|p| options.contains(p) && Some(*p) != a)
        .filter(|_| matches!(scope(), Scope::Mes | Scope::Ano));

    // Recarrega quando o placar muda (mão nova, desfazer, boi novo).
    let version = (
        view.current.as_ref().map(|c| (c.id, c.hands.len())),
        view.stats.iter().map(|s| s.maos_jogadas).sum::<u32>(),
    );
    let code = view.code.clone();
    let fetched = use_resource(use_reactive!(|a, b, version, code| async move {
        let _ = version;
        let periods: Vec<Period> = a.into_iter().chain(b).collect();
        if periods.is_empty() {
            return Ok(Vec::new());
        }
        api::room_stats(code, periods).await
    }));

    if view.stats.is_empty() {
        return rsx! {};
    }
    let unit = if scope() == Scope::Ano { "ano" } else { "mês" };
    let index_of = |p: Option<Period>| {
        p.and_then(|p| options.iter().position(|o| *o == p))
            .unwrap_or_default()
    };
    let body = match scope() {
        Scope::Tudo => stats_table(&view, view.stats.clone()),
        _ => match &*fetched.read() {
            None => rsx! { p { class: "muted", "Carregando…" } },
            Some(Err(e)) => rsx! { p { class: "error", "{error_message(e)}" } },
            Some(Ok(list)) => match (a, b, list.as_slice()) {
                (Some(a), Some(b), [ra, rb]) => {
                    compare_table(&view, (a, &ra.stats), (b, &rb.stats))
                }
                (Some(a), _, [ra, ..]) => rsx! {
                    {stats_table(&view, ra.stats.clone())}
                    if let Some(f) = &ra.firsts {
                        {firsts_list(&view, a, f)}
                    }
                },
                _ => rsx! {},
            },
        },
    };

    rsx! {
        section { class: "card",
            h2 { "Estatísticas da sala" }
            div { class: "segmented", role: "group", "aria-label": "Período",
                for (s, label) in [
                    (Scope::Tudo, "Tudo"),
                    (Scope::Dia, "Dia"),
                    (Scope::Mes, "Mês"),
                    (Scope::Ano, "Ano"),
                ]
                {
                    button {
                        key: "{label}",
                        r#type: "button",
                        class: if scope() == s { "on" },
                        onclick: move |_| {
                            scope.set(s);
                            chosen.set(None);
                            other.set(None);
                        },
                        "{label}"
                    }
                }
            }
            if scope() != Scope::Tudo {
                div { class: "period-pick",
                    select {
                        "aria-label": "Período",
                        value: "{index_of(a)}",
                        onchange: {
                            let options = options.clone();
                            move |e: FormEvent| {
                                chosen.set(e.value().parse::<usize>().ok().and_then(|i| options.get(i).copied()))
                            }
                        },
                        for (i, p) in options.iter().enumerate() {
                            option { key: "{i}", value: "{i}", "{p.label()}" }
                        }
                    }
                    if b.is_some() {
                        span { "com" }
                        select {
                            "aria-label": "Comparar com",
                            value: "{index_of(b)}",
                            onchange: {
                                let options = options.clone();
                                move |e: FormEvent| {
                                    other.set(e.value().parse::<usize>().ok().and_then(|i| options.get(i).copied()))
                                }
                            },
                            for (i, p) in options.iter().enumerate() {
                                option { key: "{i}", value: "{i}", "{p.label()}" }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "link",
                            onclick: move |_| other.set(None),
                            "Não comparar"
                        }
                    } else if matches!(scope(), Scope::Mes | Scope::Ano) && options.len() > 1 {
                        button {
                            r#type: "button",
                            class: "link",
                            onclick: {
                                let options = options.clone();
                                move |_| other.set(options.iter().copied().find(|p| Some(*p) != a))
                            },
                            "Comparar com outro {unit}"
                        }
                    }
                }
            }
            {body}
            p { class: "muted small",
                if b.is_some() {
                    "Lamb.: lambretas aplicadas. "
                } else {
                    "Lambretas e reversas: aplicadas/sofridas. "
                }
                "Nas duplas, a vitória da dupla conta para os dois parceiros."
            }
        }
    }
}

/// Primeiras vezes do ano: quem saiu, quem ganhou a primeira mão, o
/// primeiro boi, a primeira lambreta e a primeira reversa.
fn firsts_list(view: &RoomView, year: Period, f: &Firsts) -> Element {
    let names = |ids: &[i64]| view.pair_name(ids);
    let against = |m: &Moment, word: &str| {
        let others: Vec<String> = m.against.iter().map(|ids| names(ids)).collect();
        if others.is_empty() {
            String::new()
        } else {
            format!(" {word} {}", others.join(", "))
        }
    };
    let line = |m: &Option<Moment>, word: &str| match m {
        None => "Ainda não teve.".to_string(),
        Some(m) => format!(
            "{}{} (boi {}, {})",
            names(&m.by),
            against(m, word),
            m.boi_number,
            format_date(m.started_at)
        ),
    };
    let rows = [
        ("Primeira saída", line(&f.saida, "")),
        ("Primeira mão ganha", line(&f.mao, "contra")),
        ("Primeiro boi", line(&f.boi, "contra")),
        ("Primeira lambreta", line(&f.lambreta, "em")),
        ("Primeira lambreta reversa", line(&f.reversa, "em")),
    ];
    rsx! {
        h3 { "Primeiras de {year.label()}" }
        dl { class: "firsts",
            for (title, text) in rows {
                dt { key: "{title}", "{title}" }
                dd { "{text}" }
            }
        }
    }
}

/// Mais bois ganhos primeiro; empate desempata por mãos ganhas.
fn by_wins(a: &PlayerStats, b: &PlayerStats) -> std::cmp::Ordering {
    b.bois_ganhos
        .cmp(&a.bois_ganhos)
        .then(b.maos_ganhas.cmp(&a.maos_ganhas))
}

fn stats_table(view: &RoomView, mut stats: Vec<PlayerStats>) -> Element {
    if stats.is_empty() {
        return rsx! { p { class: "muted", "Ninguém jogou neste período." } };
    }
    stats.sort_by(by_wins);
    rsx! {
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
    }
}

/// Dois períodos lado a lado, do mais antigo para o mais recente: cada
/// célula mostra "antes → depois". Só três colunas, para caber no celular
/// sem rolar para o lado.
fn compare_table(
    view: &RoomView,
    a: (Period, &[PlayerStats]),
    b: (Period, &[PlayerStats]),
) -> Element {
    let ((pa, sa), (pb, sb)) = if a.0.range().0 <= b.0.range().0 {
        (a, b)
    } else {
        (b, a)
    };
    let mut rows: Vec<(PlayerStats, PlayerStats)> = Vec::new();
    for id in sb.iter().chain(sa).map(|s| s.player_id) {
        if rows.iter().any(|(_, n)| n.player_id == id) {
            continue;
        }
        let find = |list: &[PlayerStats]| {
            list.iter()
                .find(|s| s.player_id == id)
                .cloned()
                .unwrap_or(PlayerStats {
                    player_id: id,
                    ..Default::default()
                })
        };
        rows.push((find(sa), find(sb)));
    }
    rows.sort_by(|x, y| by_wins(&x.1, &y.1));
    let cell = |old: u32, new: u32| format!("{old} → {new}");
    rsx! {
        p { class: "compare-head", "{pa.label()} → {pb.label()}" }
        div { class: "table-wrap",
            table { class: "stats-table",
                thead {
                    tr {
                        th { "Jogador" }
                        th { title: "Bois ganhos", "Bois" }
                        th { title: "Mãos ganhas", "Mãos" }
                        th { title: "Lambretas aplicadas", "Lamb." }
                    }
                }
                tbody {
                    for (old, new) in rows.iter() {
                        tr { key: "{new.player_id}",
                            td { "{view.player_name(new.player_id)}" }
                            td { {cell(old.bois_ganhos, new.bois_ganhos)} }
                            td { {cell(old.maos_ganhas, new.maos_ganhas)} }
                            td { {cell(old.lambretas_aplicadas, new.lambretas_aplicadas)} }
                        }
                    }
                }
            }
        }
    }
}

/// Bois já terminados, agrupados por dia de jogo. A sala traz os 10 mais
/// recentes; "Ver mais" busca os 10 anteriores de cada vez.
#[component]
fn History(view: RoomView) -> Element {
    // Páginas mais antigas já buscadas. Ficam guardadas quando a sala
    // recarrega, porque bois antigos não mudam.
    let mut older = use_signal(Vec::<BoiView>::new);
    let mut older_more = use_signal(|| None::<bool>);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Se um boi terminou depois da busca, a primeira página andou e pode
    // repetir bois das páginas antigas; fica só o que vem depois dela.
    let first = &view.history.bois;
    let oldest_first = first.last().map(|b| b.id);
    let older_list = older.read();
    let finished: Vec<&BoiView> = first
        .iter()
        .chain(
            older_list
                .iter()
                .filter(|b| oldest_first.is_none_or(|id| b.id < id)),
        )
        .collect();
    let more = older_more().unwrap_or(view.history.more);
    if finished.is_empty() {
        return rsx! {};
    }
    let before = finished.last().map(|b| b.id).unwrap_or_default();

    let mut days: Vec<(String, Vec<&BoiView>)> = Vec::new();
    for boi in finished {
        let day = format_date(boi.started_at);
        match days.last_mut() {
            Some((d, list)) if *d == day => list.push(boi),
            _ => days.push((day, vec![boi])),
        }
    }
    let code = view.code.clone();
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
            if more {
                button {
                    r#type: "button",
                    disabled: loading(),
                    onclick: move |_| {
                        let code = code.clone();
                        async move {
                            loading.set(true);
                            match api::room_history(code, before).await {
                                Ok(page) => {
                                    older.write().extend(page.bois);
                                    older_more.set(Some(page.more));
                                    error.set(None);
                                }
                                Err(e) => error.set(Some(error_message(&e))),
                            }
                            loading.set(false);
                        }
                    },
                    if loading() { "Carregando…" } else { "Ver mais" }
                }
            }
            if let Some(e) = error() {
                p { class: "error", "{e}" }
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
