//! Tema claro, escuro ou automático (segue o sistema).
//!
//! O tema efetivo vai no atributo `data-theme` do `<html>`, que o CSS lê. Quem
//! decide é um script pequeno no `<head>`: roda antes da página aparecer,
//! então quem usa o escuro não vê a página piscar clara enquanto o app
//! carrega. A escolha fica no `localStorage` do aparelho.

use dioxus::prelude::*;

/// Vai no `<head>` (ver `App`). Expõe `window.placarTema(escolha)` para trocar
/// e `window.placarTemaAtual()` para ler a escolha ("auto", "claro" ou
/// "escuro").
pub const THEME_SCRIPT: &str = r#"(function () {
  var escolha = "auto";
  try { escolha = localStorage.getItem("tema") || "auto"; } catch (e) {}
  var sistema = matchMedia("(prefers-color-scheme: dark)");
  function aplicar() {
    var escuro = escolha === "escuro" || (escolha === "auto" && sistema.matches);
    document.documentElement.dataset.theme = escuro ? "dark" : "light";
  }
  aplicar();
  sistema.addEventListener("change", aplicar);
  window.placarTema = function (nova) {
    escolha = nova;
    try {
      if (nova === "auto") localStorage.removeItem("tema");
      else localStorage.setItem("tema", nova);
    } catch (e) {}
    aplicar();
  };
  window.placarTemaAtual = function () { return escolha; };
})();"#;

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    Auto,
    Light,
    Dark,
}

impl Theme {
    fn key(self) -> &'static str {
        match self {
            Theme::Auto => "auto",
            Theme::Light => "claro",
            Theme::Dark => "escuro",
        }
    }

    fn parse(s: &str) -> Theme {
        match s {
            "claro" => Theme::Light,
            "escuro" => Theme::Dark,
            _ => Theme::Auto,
        }
    }

    fn next(self) -> Theme {
        match self {
            Theme::Auto => Theme::Light,
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Auto,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Theme::Auto => "◐",
            Theme::Light => "☀",
            Theme::Dark => "☾",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Theme::Auto => "Auto",
            Theme::Light => "Claro",
            Theme::Dark => "Escuro",
        }
    }
}

/// Botão que alterna entre automático, claro e escuro, mostrando o atual.
#[component]
pub fn ThemeButton() -> Element {
    let mut theme = use_signal(|| Theme::Auto);
    // A escolha guardada só existe no navegador: lê depois de montar.
    use_effect(move || {
        spawn(async move {
            let saved =
                document::eval("return window.placarTemaAtual ? window.placarTemaAtual() : 'auto'")
                    .await
                    .ok()
                    .and_then(|v| v.as_str().map(Theme::parse));
            if let Some(t) = saved {
                theme.set(t);
            }
        });
    });
    let current = theme();
    rsx! {
        button {
            r#type: "button",
            class: "tool theme",
            title: "Tema: {current.label()}. Toque para trocar.",
            "aria-label": "Tema: {current.label()}. Toque para trocar.",
            onclick: move |_| {
                let next = theme().next();
                theme.set(next);
                document::eval(&format!("window.placarTema && window.placarTema('{}')", next.key()));
            },
            span { "aria-hidden": "true", "{current.icon()}" }
            span { class: "tool-text", "{current.label()}" }
        }
    }
}
