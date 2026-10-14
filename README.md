# Placar de Dominó

Placar de dominó para jogar em família: marca as mãos de cada boi, mostra o
placar ao vivo para quem está na mesa ou longe dela e guarda o histórico e
as estatísticas de cada sala.

**No ar:** https://placar-276818208775.southamerica-east1.run.app

Feito em [Dioxus](https://dioxuslabs.com) 0.7 (Rust no servidor e no
navegador, via WebAssembly), com o banco no [Turso](https://turso.tech). O
projeto também é um jeito de aprender Dioxus.

## O que faz

- **Salas** em duplas ou individual (2 a 4 jogadores). Quem tem o código da
  sala acompanha o placar ao vivo, sem precisar de conta.
- **Placar em cruz:** cada boi é uma cruz de 4 braços, e cada mão ganha
  pinta um braço. Reconhece lambreta (4 a 0) e lambreta reversa, fechado
  empatado nas duplas e a saída pela maior carroça. As regras da família
  estão em [`docs/REGRAS.md`](docs/REGRAS.md).
- **Desfazer** a última mão, e trocar os jogadores entre um boi e outro.
- **Histórico** por dia, com a gaveta de cada boi.
- **Estatísticas** por período (dia, mês, ano ou tudo), comparação entre
  dois meses ou anos, destaques, as primeiras do ano e as viradas.
- **Contas** com senha, e-mail confirmado por código, entrada com código e
  link de troca de senha gerado pelo administrador.
- **Ajuda em cada tela** (o "?" no topo) e tema claro, escuro ou automático.

## Rodar localmente

Precisa de Rust (com o alvo `wasm32-unknown-unknown`) e do
[`dx`](https://dioxuslabs.com/learn/0.7/getting_started) 0.7.10.

```sh
rustup target add wasm32-unknown-unknown
cargo binstall dioxus-cli@0.7.10   # ou: cargo install dioxus-cli@0.7.10
dx serve --platform web --port 8080
```

Sem configuração, o banco é um arquivo SQLite local (`placar.db`). Variáveis
opcionais:

| Variável | Para quê |
|---|---|
| `PLACAR_DB` | Caminho do arquivo SQLite local. |
| `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN` | Usar o Turso em vez do arquivo local. |
| `PLACAR_ADMIN` | Usuário que gera links de troca de senha. |
| `PLACAR_SMTP_USER`, `PLACAR_SMTP_PASSWORD` | Conta do Gmail do app e uma senha de app dela, para mandar os códigos por e-mail. Sem elas, os e-mails só vão para o log. |
| `PLACAR_SMTP_HOST` | Outro servidor SMTP (padrão: `smtp.gmail.com`). |
| `PLACAR_MAILBOX` | Sem SMTP, arquivo onde os e-mails são escritos (o teste da interface lê os códigos dali). |
| `PLACAR_HTTPS` | Cookie de sessão só por HTTPS (produção). |

## Testes

```sh
cargo fmt --check
cargo clippy --features server --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
cargo test --features server
python3 tools/ui_test.py   # interface no Safari, com o dx serve rodando
```

O teste da interface controla o Safari via WebDriver e precisa de "Permitir
Automação Remota" ligado no menu Desenvolvedor.

## Organização

| Onde | O quê |
|---|---|
| `src/rules.rs` | Regras do jogo: boi, lambreta, saída, estatísticas. |
| `src/room_rules.rs` | O que pode ou não ser feito numa sala. |
| `src/model.rs`, `src/views.rs`, `src/period.rs` | Tipos do domínio, o que vai para a tela, datas no fuso de Brasília. |
| `src/server.rs`, `src/accounts.rs` | Banco, sessões, contas e códigos. |
| `src/api/` | Funções do servidor chamadas pela interface. |
| `src/ui/` | Telas e componentes. |
| `assets/main.css` | Estilo, com os temas claro e escuro. |
| `tools/` | Teste da interface e páginas auxiliares. |

## Deploy

Cada push na `main` com o CI verde publica no Google Cloud Run (São Paulo)
pelo GitHub Actions. Detalhes em [`docs/DEPLOY.md`](docs/DEPLOY.md).

## Documentos

- [`docs/ESTADO.md`](docs/ESTADO.md): estado atual e como retomar o trabalho.
- [`docs/BACKLOG.md`](docs/BACKLOG.md): o que falta, na ordem.
- [`docs/REGRAS.md`](docs/REGRAS.md): regras do jogo.
- [`docs/DEPLOY.md`](docs/DEPLOY.md): hospedagem e publicação.
