# Deploy

Escolha de 2026-10-14: **Google Cloud Run** em São Paulo
(`southamerica-east1`) com o banco no **Turso**, também em São Paulo.

No ar desde 2026-10-14: https://placar-276818208775.southamerica-east1.run.app

- Projeto do Google Cloud: `domino-237616`. Usar a configuração `placar` do
  gcloud (`CLOUDSDK_ACTIVE_CONFIG_NAME=placar`).
- Banco: `placar`, no grupo `saopaulo` do Turso (`aws-sa-east-1`; o grupo
  `default` da conta só aceita a Virgínia).
- Token do banco no Secret Manager (`turso-token`), lido pelo Cloud Run.
- Imagens no Artifact Registry (`cloud-run-source-deploy`) com limpeza
  automática: ficam só as 3 mais recentes (cota grátis de 0,5 GB).

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
| `PLACAR_SMTP_USER` | Gmail do app, remetente dos códigos. |
| `PLACAR_SMTP_PASSWORD` | Senha de app desse Gmail, no Secret Manager (`smtp-password`). |
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
gcloud services enable run.googleapis.com cloudbuild.googleapis.com artifactregistry.googleapis.com secretmanager.googleapis.com

turso group create saopaulo --location aws-sa-east-1
turso db create placar --group saopaulo
# Token direto para o Secret Manager, sem passar pela tela.
turso db tokens create placar | tr -d '\n' | gcloud secrets create turso-token --data-file=-
gcloud secrets add-iam-policy-binding turso-token \
  --member=serviceAccount:<número do projeto>-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor

# Compila no Cloud Build (o Mac tem pouco disco para o Docker) e publica.
# A compilação passa de 10 min: `gcloud config set builds/timeout 3600`.
gcloud run deploy placar --source . --region southamerica-east1 \
  --allow-unauthenticated --max-instances 1 --cpu 1 --memory 512Mi \
  --timeout 3600 \
  --set-env-vars PLACAR_HTTPS=1,TURSO_DATABASE_URL=libsql://placar-dede999.aws-sa-east-1.turso.io \
  --set-secrets TURSO_AUTH_TOKEN=turso-token:latest

# E-mail: a senha de app do Gmail vai direto para o Secret Manager, digitada
# no terminal de quem cuida do app (nunca no chat nem em arquivo).
read -s SENHA && printf %s "$SENHA" | tr -d ' ' | gcloud secrets create smtp-password --data-file=- && unset SENHA
gcloud secrets add-iam-policy-binding smtp-password \
  --member=serviceAccount:<número do projeto>-compute@developer.gserviceaccount.com \
  --role=roles/secretmanager.secretAccessor
gcloud run services update placar --region southamerica-east1 \
  --update-env-vars PLACAR_SMTP_USER=<gmail do app> \
  --update-secrets PLACAR_SMTP_PASSWORD=smtp-password:latest

# Administrador, depois de criar a conta no app (sem recompilar):
gcloud run services update placar --region southamerica-east1 --update-env-vars PLACAR_ADMIN=<usuário>
```

Deploys seguintes: automáticos, ver "CD" abaixo. À mão, o mesmo
`gcloud run deploy --source .` funciona (as variáveis e o segredo
continuam), mas leva uns 15 min, porque o Cloud Build compila tudo do zero.

`--timeout 3600`: a conexão ao vivo cai depois de 1 hora e a página
reconecta sozinha.

## CD

`.github/workflows/deploy.yml`: quando o CI passa num push na `main`, o
GitHub Actions compila com `dx bundle` (com cache), monta a imagem com
`runtime.Dockerfile` (só a etapa final do `Dockerfile`), manda para o
Artifact Registry e troca a imagem do serviço no Cloud Run.

Commit só de docs (`docs/**`, `**.md`, `tools/**`, `.gitignore`) não roda o
CI (`paths-ignore` em `ci.yml`) e, por isso, não faz deploy: o gatilho do
deploy (`workflow_run`) não aceita filtro de arquivos. Se um dia a `main`
exigir o CI aprovado para aceitar PR, um PR só de docs ficaria esperando um
CI que nunca roda; aí trocar o filtro por um passo que verifica os arquivos
mudados.

Login sem chave (Workload Identity Federation), criado em 2026-10-14:

- Pool `github`, provedor `github-oidc`. Condição: só
  `dede999/family-domino-score` na `refs/heads/main`.
- Conta `github-deploy@domino-237616.iam.gserviceaccount.com`, com
  `run.developer` no projeto, `artifactregistry.writer` só no repositório
  `cloud-run-source-deploy` e `iam.serviceAccountUser` na conta do Cloud
  Run (para publicar revisões que rodam como ela).

## Testado

- 2026-10-14: `dx bundle --release` no Mac e o `server` rodando com
  `IP=0.0.0.0 PORT=8091 PLACAR_HTTPS=1`: entrega a página e o cookie sai com
  `Secure`.
- 2026-10-14, em produção: a página abre, a interface fica pronta em ~1 s
  no Safari (wasm de ~3 MB), e a busca de sala consulta o Turso em São
  Paulo. O `dx` pré-compilado pede glibc 2.39, por isso as imagens são
  Debian trixie.
