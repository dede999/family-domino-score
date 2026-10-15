# Estado do desenvolvimento

Arquivo para retomar o trabalho sem depender do histórico da conversa.
Atualizar ao fim de cada etapa. Atualizado em 2026-10-13.

Cliques perdidos (resolvido em 2026-10-13): quando a janela do Safari
automatizado perde o foco (alguém usando outra janela do Mac durante o
teste), o WebDriver responde que clicou e a página não recebe nada. Era a
causa das falhas "intermitentes". Confirmado: nessas horas
`document.hasFocus()` é falso, e o app continua respondendo a cliques por
JavaScript; o mesmo acontece com o build de produção, então não é do `dx`.
`click()` no teste confere se o clique chegou e, se não, repete pelo
JavaScript com um AVISO. Muitos avisos = janela sem foco; para um teste
"de verdade", deixar o Safari na frente e não mexer no Mac.

## O que é

Placar de dominó para família e amigos. Feito em Dioxus 0.7.10 fullstack com
Turso (libsql). Objetivo principal: aprender Dioxus. As regras do jogo estão em
`docs/REGRAS.md`.

## Pronto

- Backend (`src/server.rs`, `src/api/`, `src/accounts.rs`): cadastro e login
  (argon2 + cookie), e-mail confirmado por código, entrar com código, troca
  de senha por código ou por link do administrador,
  salas, jogadores avulsos, mãos, desfazer, edição de jogadores entre bois,
  eventos ao vivo via SSE. E-mails pelo Gmail (SMTP) em produção.
- Regras (`src/rules.rs`): boi até 4, lambreta, lambreta reversa, fechado em
  duplas com empate, saída pela carroça.
- Interface (`src/ui/`): início com login e cadastro, nova sala, sala com
  placar (cruz SVG), mãos do boi, estatísticas e histórico.
- Testado pela API (`tools/api_test.sh`) e pela interface no Safari
  (`tools/ui_test.py`).

## Em andamento

Nada. O app está no ar (ver `docs/DEPLOY.md`). Próximo item: Play Store
(item 21 do `BACKLOG.md`).

## Teste da interface

`python3 tools/ui_test.py`, com o servidor rodando. Controla o Safari via
WebDriver, sem dependências, e cobre cadastro, sala, boi completo, desfazer,
tempo real, estatísticas, histórico paginado, visitante, ajuda e tema. Passou inteiro
em 2026-10-13. Precisa
de "Permitir Automação Remota" ligado no menu Desenvolvedor do Safari.

O teste espera a marca `data-pronto` no `<html>`, posta por um efeito em
`App` (`src/main.rs`) depois da hidratação. Sem isso ele clicava antes de o
WebAssembly assumir a página, e o clique se perdia. Foi isso que pareceu um
bug do `onsubmit` no Safari, mas não era.

## CI

`.github/workflows/ci.yml`: fmt, clippy (servidor e navegador, avisos viram
erro) e testes. Antes de commitar, rodar o mesmo localmente:
`cargo fmt && cargo clippy --features server --all-targets -- -D warnings`.

## Falta

- Itens de `docs/BACKLOG.md`.
- Deploy: ver `docs/DEPLOY.md`.

## Regras de trabalho

- Commits o mais atômicos possível: uma mudança lógica por commit.
- `target/` pode ser apagado de vez em quando (`cargo clean`) para liberar
  espaço. Ele fica no KINGSTON, então isso não ajuda o disco interno. A
  compilação seguinte é completa e leva alguns minutos.
- Nada novo começa antes de terminar o que está em andamento (ver
  `BACKLOG.md`).

## Como rodar

O disco interno do Mac tem pouco espaço livre, então tudo que é temporário
vai para o KINGSTON:

```sh
TMPDIR=/Volumes/KINGSTON/.cargo-build/tmp CARGO_BUILD_JOBS=4 \
PLACAR_DB=/Volumes/KINGSTON/.cargo-build/tmp/teste.db \
PLACAR_MAILBOX=/Volumes/KINGSTON/.cargo-build/tmp/mailbox.txt \
dx serve --platform web --port 8080 --open false --interactive false --hot-reload false
```

`PLACAR_MAILBOX` é onde os e-mails vão parar sem SMTP configurado
(produção envia pelo Gmail, ver `DEPLOY.md`) (o teste da interface lê os códigos dali). `PLACAR_ADMIN=<usuário>`
faz dessa conta a administradora, que gera links de troca de senha em
"Minha conta".

Com `--interactive false` o `dx` não recompila sozinho. É preciso reiniciá-lo
depois de mudar o código. A primeira compilação leva uns 2 minutos.
