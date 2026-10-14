# Deploy

Escolha de 2026-10-14: **Google Cloud Run** em São Paulo
(`southamerica-east1`) com o banco no **Turso**, também em São Paulo.

## Por que assim

- O app é um processo só: o binário `server` entrega a interface (`public/`)
  e responde a API. Não há front separado para hospedar. Separar o front
  (GitHub Pages etc.) faria o cookie de login virar "de terceiros", que o
  Safari bloqueia.
- Cloud Run desliga quando ninguém usa e um binário Rust acorda em menos de
  um segundo. A cota grátis mensal (por conta de cobrança, não por região) é
  de 180 mil vCPU-segundos, 360 mil GiB-segundos e 2 milhões de requisições.
- Servidor e banco na mesma cidade: cada tela faz várias consultas.
- Descartados: Fly.io (uns US$ 3,50/mês em `gru`, sem cota grátis; boa
  reserva se o Cloud Run der trabalho), Railway (sem região no Brasil),
  Render grátis (dorme e leva quase 1 minuto para acordar), Appwrite (plano
  grátis pausa o projeto após 7 dias sem uso do painel e apaga após 90;
  seria reescrever o backend).

## Cuidado com a cobrança

A atualização ao vivo da sala é uma conexão aberta (SSE), e o Cloud Run
cobra o tempo em que há alguma requisição aberta. Com 1 vCPU, a cota grátis
dá umas **50 horas por mês de "alguém com a sala aberta"**. Passando disso,
uns US$ 0,12 por hora. Por isso:

- A página fecha a conexão quando fica escondida (outra aba, app em segundo
  plano, tela apagada) e reabre ao voltar.
- O servidor manda um sinal de vida a cada 25 s, para soltar conexões de
  quem caiu sem avisar.
- `--max-instances 1`: os avisos de mudança ficam na memória do processo
  (`CHANNELS` em `src/server.rs`). Com duas instâncias, quem estivesse
  ligado na outra não seria avisado.
- Criar um **alerta de orçamento** baixo (ex.: US$ 1) no Cloud Billing.

## Variáveis

| Variável | Para quê |
|---|---|
| `TURSO_DATABASE_URL`, `TURSO_AUTH_TOKEN` | Banco no Turso. Sem elas, usa um arquivo SQLite local. |
| `PLACAR_HTTPS=1` | Cookie de sessão com `Secure`. |
| `PLACAR_ADMIN` | Conta que gera links de troca de senha. |
| `IP`, `PORT` | Já definidas no `Dockerfile` (`0.0.0.0:8080`). |

## Passos

O que só o André pode fazer (contas e cartão):

1. Conta no Google Cloud com cobrança ativada (pede cartão) e um projeto.
2. Conta no Turso.
3. `brew install --cask google-cloud-sdk` e `brew install tursodatabase/tap/turso`.
4. `gcloud auth login` e `turso auth login` (abrem o navegador).

Depois:

```sh
gcloud config set project <projeto>
gcloud services enable run.googleapis.com cloudbuild.googleapis.com artifactregistry.googleapis.com

turso db locations                      # conferir o código de São Paulo
turso db create placar --location <são-paulo>
turso db show placar --url              # TURSO_DATABASE_URL
turso db tokens create placar           # TURSO_AUTH_TOKEN

# Compila no Cloud Build (o Mac tem pouco disco para o Docker) e publica.
gcloud run deploy placar --source . --region southamerica-east1 \
  --allow-unauthenticated --max-instances 1 --cpu 1 --memory 512Mi \
  --timeout 3600 \
  --set-env-vars PLACAR_HTTPS=1,PLACAR_ADMIN=<usuário>,TURSO_DATABASE_URL=<url>,TURSO_AUTH_TOKEN=<token>
```

`--timeout 3600`: a conexão ao vivo cai depois de 1 hora e a página
reconecta sozinha.

## Testado

- 2026-10-14: `dx bundle --release` no Mac e o `server` rodando com
  `IP=0.0.0.0 PORT=8091 PLACAR_HTTPS=1`: entrega a página e o cookie sai com
  `Secure`. A imagem Docker ainda não foi construída (será no Cloud Build).
