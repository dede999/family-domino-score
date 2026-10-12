# Estado do desenvolvimento

Arquivo para retomar o trabalho sem depender do histórico da conversa.
Atualizar ao fim de cada etapa. Atualizado em 2026-10-12.

## O que é

Placar de dominó para família e amigos. Feito em Dioxus 0.7.10 fullstack com
Turso (libsql). Objetivo principal: aprender Dioxus. As regras do jogo estão em
`docs/REGRAS.md`.

## Pronto

- Backend (`src/server.rs`, `src/api.rs`): cadastro e login (argon2 + cookie),
  salas, jogadores avulsos, mãos, desfazer, edição de jogadores entre bois,
  eventos ao vivo via SSE.
- Regras (`src/rules.rs`): boi até 4, lambreta, lambreta reversa, fechado em
  duplas com empate, saída pela carroça.
- Interface (`src/ui/`): início com login e cadastro, nova sala, sala com
  placar (cruz SVG), mãos do boi, estatísticas e histórico.
- Testado pela API (`tools/api_test.sh`) e pela interface no Safari
  (`tools/ui_test.py`).

## Em andamento

Nada. Próximo item: ver `BACKLOG.md` → "Ordem atual".

## Teste da interface

`python3 tools/ui_test.py`, com o servidor rodando. Controla o Safari via
WebDriver, sem dependências, e cobre cadastro, sala, boi completo, desfazer,
tempo real, estatísticas e visitante. Passou inteiro em 2026-10-12. Precisa
de "Permitir Automação Remota" ligado no menu Desenvolvedor do Safari.

O teste espera a marca `data-pronto` no `<html>`, posta por um efeito em
`App` (`src/main.rs`) depois da hidratação. Sem isso ele clicava antes de o
WebAssembly assumir a página, e o clique se perdia. Foi isso que pareceu um
bug do `onsubmit` no Safari, mas não era.

## Falta

- Itens de `docs/BACKLOG.md`.
- Deploy: banco no Turso e hospedagem.

## Como rodar

O disco interno do Mac tem pouco espaço livre, então tudo que é temporário
vai para o KINGSTON:

```sh
TMPDIR=/Volumes/KINGSTON/.cargo-build/tmp CARGO_BUILD_JOBS=4 \
PLACAR_DB=/Volumes/KINGSTON/.cargo-build/tmp/teste.db \
dx serve --platform web --port 8080 --open false --interactive false --hot-reload false
```

Com `--interactive false` o `dx` não recompila sozinho. É preciso reiniciá-lo
depois de mudar o código. A primeira compilação leva uns 2 minutos.
