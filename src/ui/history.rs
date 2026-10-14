//! Histórico da sala: uma sanfona por dia, com um cartão por boi. Tocar no
//! cartão abre uma gaveta de baixo para cima com o boi inteiro.

use super::drawer::Drawer;
use super::room::{Crosses, FinalResult, HandStrip};
use crate::api::{self, error_message};
use crate::model::BoiView;
use crate::palette;
use crate::period::{Period, format_date};
use crate::rules::SideResult;
use crate::views::{DaySummary, RoomView};
use dioxus::prelude::*;

/// Dias com boi terminado, do mais recente para o mais antigo. A sala traz
/// os primeiros dias só com o resumo; "Ver mais" busca dias mais antigos.
#[component]
pub fn History(view: RoomView) -> Element {
    // Dias mais antigos já buscados. Ficam guardados quando a sala recarrega,
    // porque dias passados não mudam.
    let mut older = use_signal(Vec::<DaySummary>::new);
    let mut older_more = use_signal(|| None::<bool>);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut selected = use_signal(|| None::<BoiView>);

    let first = &view.history.days;
    let oldest_first = first.last().map(|d| d.day.range().0);
    let older_list = older.read();
    let days: Vec<&DaySummary> = first
        .iter()
        .chain(
            older_list
                .iter()
                .filter(|d| oldest_first.is_none_or(|t| d.day.range().0 < t)),
        )
        .collect();
    if days.is_empty() {
        return rsx! {};
    }
    let more = older_more().unwrap_or(view.history.more);
    let before = days.last().map(|d| d.day);
    let code = view.code.clone();

    rsx! {
        section { class: "card",
            h2 { "Histórico" }
            div { class: "days",
                for (i, day) in days.iter().enumerate() {
                    Day {
                        key: "{day.day.label()}",
                        view: view.clone(),
                        summary: (*day).clone(),
                        open_at_start: i == 0,
                        on_select: move |boi| selected.set(Some(boi)),
                    }
                }
            }
            if more {
                button {
                    r#type: "button",
                    class: "secondary",
                    disabled: loading(),
                    onclick: move |_| {
                        let code = code.clone();
                        async move {
                            let Some(before) = before else { return };
                            loading.set(true);
                            match api::room_history(code, before).await {
                                Ok(page) => {
                                    older.write().extend(page.days);
                                    older_more.set(Some(page.more));
                                    error.set(None);
                                }
                                Err(e) => error.set(Some(error_message(&e))),
                            }
                            loading.set(false);
                        }
                    },
                    if loading() { "Carregando…" } else { "Ver mais dias" }
                }
            }
            if let Some(e) = error() {
                p { class: "error", "{e}" }
            }
        }
        if let Some(boi) = selected() {
            BoiDrawer { view: view.clone(), boi, on_close: move |_| selected.set(None) }
        }
    }
}

/// Resumo do dia: "8 bois · André & Sobrinho 5 × 3 Pai & Tio". Com mais de
/// dois lados: "Ana 3 · Bia 2 · Caio 1".
fn day_line(view: &RoomView, d: &DaySummary) -> String {
    let n = if d.bois == 1 {
        "1 boi".to_string()
    } else {
        format!("{} bois", d.bois)
    };
    let name = |ids: &[i64]| view.pair_name(ids);
    let score = match d.sides.as_slice() {
        [a, b] => format!(
            "{} {} × {} {}",
            name(&a.players),
            a.won,
            b.won,
            name(&b.players)
        ),
        sides => sides
            .iter()
            .map(|t| format!("{} {}", name(&t.players), t.won))
            .collect::<Vec<_>>()
            .join(" · "),
    };
    format!("{n} · {score}")
}

/// Um dia da sanfona. Os bois só são buscados quando o dia está aberto, e de
/// novo quando o número de bois do dia muda (um boi terminou agora).
#[component]
fn Day(
    view: RoomView,
    summary: DaySummary,
    open_at_start: bool,
    on_select: EventHandler<BoiView>,
) -> Element {
    let mut open = use_signal(|| open_at_start);
    let is_open = open();
    let code = view.code.clone();
    let day = summary.day;
    let count = summary.bois;
    let bois = use_resource(use_reactive!(|code, day, is_open, count| async move {
        let _ = count;
        if !is_open {
            return Ok(Vec::new());
        }
        api::room_day(code, day).await
    }));

    rsx! {
        div { class: "day",
            button {
                r#type: "button",
                class: "day-head",
                "aria-expanded": is_open,
                onclick: move |_| open.toggle(),
                span { class: "day-date", "{day_title(day, view.today)}" }
                span { class: "day-line", "{day_line(&view, &summary)}" }
                span { class: "day-chevron", "aria-hidden": "true", if is_open { "▴" } else { "▾" } }
            }
            if is_open {
                match &*bois.read() {
                    None => rsx! { p { class: "muted", "Carregando…" } },
                    Some(Err(e)) => rsx! { p { class: "error", "{error_message(e)}" } },
                    Some(Ok(list)) => rsx! {
                        div { class: "boi-grid",
                            for boi in list.iter() {
                                BoiCard {
                                    key: "{boi.id}",
                                    view: view.clone(),
                                    boi: boi.clone(),
                                    on_select,
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

/// "Hoje", "Ontem" ou a data. `today` vem do servidor (fuso de Brasília).
fn day_title(day: Period, today: Period) -> String {
    let (start, _) = today.range();
    if day == today {
        "Hoje".into()
    } else if day == Period::day_of(start - 1) {
        "Ontem".into()
    } else {
        day.label()
    }
}

/// Cartão de um boi: número, quem ganhou, placar e lambreta.
#[component]
fn BoiCard(view: RoomView, boi: BoiView, on_select: EventHandler<BoiView>) -> Element {
    let state = view.state(&boi);
    let Some(winner) = state.winner else {
        return rsx! {};
    };
    let colors = view.side_colors(&boi);
    let others: Vec<String> = (0..state.scores.len())
        .filter(|&s| s != winner)
        .map(|s| state.scores[s].to_string())
        .collect();
    let tag = if state.results.contains(&Some(SideResult::Lambreta)) {
        Some("lambreta")
    } else if state.results.contains(&Some(SideResult::LambretaReversa)) {
        Some("reversa")
    } else {
        None
    };
    let b = boi.clone();
    rsx! {
        button {
            r#type: "button",
            class: "boi-card",
            style: palette::style(colors[winner]),
            onclick: move |_| on_select.call(b.clone()),
            span { class: "boi-num", "Boi {boi.number}" }
            strong { class: "boi-winner", "{view.side_name(&boi, winner)}" }
            span { class: "boi-score", "{state.scores[winner]} × {others.join(\", \")}" }
            if let Some(tag) = tag {
                span { class: "boi-tag", "{tag}" }
            }
        }
    }
}

/// Gaveta de baixo para cima com o boi inteiro: cruzes, resultado, quem saiu
/// na primeira mão válida e a fileira de mãos.
#[component]
fn BoiDrawer(view: RoomView, boi: BoiView, on_close: EventHandler<()>) -> Element {
    let colors = view.side_colors(&boi);
    // Primeira mão válida: a primeira que não terminou empatada.
    let first_starter = boi
        .hands
        .iter()
        .find(|h| h.winner_side.is_some())
        .and_then(|h| boi.seating.get(h.starter_seat))
        .map(|&id| view.player_name(id));
    rsx! {
        Drawer {
            title: "Boi nº {boi.number}",
            subtitle: format_date(boi.started_at),
            on_close,
            Crosses { view: view.clone(), boi: boi.clone() }
            FinalResult { view: view.clone(), boi: boi.clone() }
            dl { class: "stats",
                dt { "Saiu na primeira mão válida" }
                dd { {first_starter.unwrap_or_else(|| "—".into())} }
                dt { "Mãos" }
                dd { "{boi.hands.len()}" }
            }
            HandStrip { view: view.clone(), boi: boi.clone(), colors }
        }
    }
}
