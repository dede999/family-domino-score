# Backlog

Ideias com a decisão tomada. Marcar `[x]` quando pronto. Itens marcados com
**(em discussão)** ainda esperam a resposta do André.

**Regra de trabalho (decidida em 2026-10-12):** nada novo começa antes de
terminar o que já estava em andamento. A ordem é a desta lista e de
`ESTADO.md` → "Em andamento". Ideia nova entra no fim da fila, não na frente.

## Ordem atual

1. ~~Bug do `onsubmit` no Safari~~: não era bug, ver `ESTADO.md`.
2. [x] Página de teste das cores (`tools/cores.html`, publicada em
   https://claude.ai/artifact/Dm8kXioB2AZ99XehNq84S8). Decidido em 2026-10-12
   com os valores padrão da página: casca L=0,50 (claro) / 0,74 (escuro),
   miolo 25% de cor, casca com espessura 4,5 (vazia continua 3). Paleta: azul,
   ciano, turquesa, verde, lima, roxo, magenta, marrom (matiz, croma e ajuste
   de claridade em `PALETTE` dentro de `tools/cores.html`). Laranja saiu.
   **Preenchimento da cruz no sentido anti-horário**: cima, esquerda, baixo,
   direita.
3. [x] Botões apagados até o app carregar (ver Infra).
4. [x] Espaçamento, cores e bolinhas.
5. [x] Mãos do boi como fileira de quadrados. Ajuste de 2026-10-13:
   quadrados de 38px e, nas duplas, retângulo com as iniciais da dupla.
6. [x] Histórico com "ver mais".
7. [x] CI (ver Infra). Passou na frente das estatísticas a pedido do
   André. CD só depois do deploy.
8. [x] Estatísticas por período.
9. [x] "Viradas" (os extras ficam para depois, não são obrigatórios).
10. [x] Regras fora dos handlers (ver Arquitetura).
11. [x] Recuperar senha (ver Infra), junto com separar as contas do placar.
12. [x] Separar `src/model.rs` (ver Arquitetura).
13. [x] Histórico em cartões.
14. [x] Deploy no Cloud Run + Turso (ver `DEPLOY.md`). No ar em 2026-10-14.
    `PLACAR_ADMIN=andre_luiz` configurado.
15. [x] Guia de uso: "?" por tela e troca de tema no topo (ver a seção própria).
16. [x] CD pelo GitHub Actions (ver Infra). Pedido em 2026-10-14, passou
    na frente do guia.
17. [x] Comprimir a interface (ver Infra). Fila pedida em 2026-10-14:
18. [x] README.md.
19. [x] Tabela de estatísticas sem rolar para o lado no celular: até 560 px,
    um cartão por jogador.
20. [x] Envio de e-mail de verdade por SMTP (Gmail), no ar em 2026-10-15.
    Os primeiros e-mails caem no spam (conta nova); marcar "Não é spam".
21. [ ] Play Store (ver "Futuro: distribuição").

## Visual

- [x] **Revisão de espaçamento e coerência visual.** Escala `--sp-1..6`
  (4/8/12/16/24/48) em `assets/main.css`. O espaço entre blocos vem do `gap`
  do contêiner (`.page`, `.card`); elementos não têm margem própria. Mesmo
  cabeçalho em todas as telas (`← Início` + `.page-head`). Linhas de jogador
  em grade fixa (o ↑ da primeira fica desativado). Corrigido "Sai: Paitrocar".

- [x] **Cor por jogador, editável.** Feito: `src/palette.rs`, tabela `pairs`,
  `bois.colors`, cartão "Cores das duplas" e bolinha de cor no editor de
  jogadores.
  - Escolha numa paleta fixa de 8 cores, não num seletor livre. A paleta tem
    cores distintas entre si e legíveis no tema claro e no escuro.
  - A cor é guardada como índice da paleta (não como hex), e a paleta fica numa
    lista única no código. Assim dá para crescer para 12 ou 16 depois sem
    migrar dados. Hoje 8 basta (no máximo 4 lados por boi).
  - Dourado, vermelho e o cinza-azulado do empate ficam fora da paleta, porque
    têm significado próprio (vitória, lambreta, empate).
  - Ao criar a sala, o app distribui cores diferentes. O dono pode trocar.
  - Guardar a cor no jogador da sala (coluna nova em `players`).
  - Duplas (decidido): a sala guarda os jogadores, e cada dupla que se forma
    (par sem ordem) escolhe uma cor **primária** e uma **secundária**,
    diferentes entre si. Jogador em modo individual só tem a primária. O nome
    da dupla segue a ordem alfabética ("Ana & Carlos"), comparando sem
    acento e sem maiúsculas, para "Álvaro" não ir para o fim. Em sala de
    duplas os jogadores não têm cor individual.
  - Uso da secundária: só como reserva. Se as duas duplas de um boi tiverem a
    mesma primária, quem escolheu aquela cor por último usa a secundária. Se
    ainda assim houver choque, o app pede uma troca ao começar o boi. A
    secundária não aparece junto da primária, para não competir com a
    diferença entre batida e fechado.
  - Cada boi guarda as cores usadas quando começou. O histórico não muda se
    alguém trocar de cor depois.

- [x] **Bolinhas na cor do lado.** Feito: `Cross` em `src/ui/mod.rs`.
  - Batida: bolinha cheia na cor.
  - Fechado (decidido: casca escura + miolo claro; o grau de claridade sai do
    teste de contraste na página de cores): só o tom claro é difícil de ver em
    cores claras e no sol, e só o contorno fica fraco em tamanho pequeno. Fica:
    miolo no tom claro + borda grossa (~25% do raio) na cor cheia. O tom
    claro sai de `color-mix(in oklch, <cor> 40%, var(--card))`. Vale igual
    para os quadrados. Testar na tela antes de fechar. Atenção: hoje a casa
    vazia da cruz já é um anel (só contorno), então o fechado não pode ser só
    contorno de jeito nenhum.
  - Fim do boi: as bolinhas do vencedor ficam douradas, mantendo a diferença
    entre cheia e anel. Isso já existe em parte (`.cross.winner`).
  - Perdedores (decidido): as bolinhas ficam acinzentadas, e a cruz de quem
    levou lambreta ganha destaque em vermelho (contorno ou braços). Motivo: com
    3 ou 4 jogadores, pintar todos os perdedores de vermelho apagaria quem é
    quem, e quem levou lambreta não tem bolinha para pintar.

- [x] **Mãos do boi como fileira de quadrados, no topo e em destaque.** Feito:
  `HandStrip` em `src/ui/room.rs`.
  - Trocar a lista de texto (`HandList`, hoje fechada dentro de `<details>`)
    por uma fileira de quadrados com cantos bem arredondados (~30% do lado),
    na ordem em que as mãos aconteceram.
  - Quadrado com a mesma regra das bolinhas: cheio na cor (batida) ou
    claro/anel (fechado). Empate: quadrado cinza-azulado (slate, fora da
    paleta) com "=". Decidido: "=" e não "E", que confundiria com a inicial de
    um jogador (ex.: Ernesto).
  - A última mão fica em destaque (maior ou com contorno). Tocar num quadrado
    mostra o detalhe (quem saiu, quem venceu, como).
  - Posição: logo abaixo das cruzes, acima dos botões de marcar.

## Histórico

- [x] **Mostrar só os últimos N bois no histórico.** Feito:
  `history_page` em `src/model.rs`, `api::room_history` e `History` em
  `src/ui/room.rs`. As estatísticas da sala agora vêm do servidor
  (`RoomView.stats`). O servidor ainda lê todos os bois do banco a cada
  atualização; só a transferência ficou menor.
  - Mostra 10 e um botão "ver mais" carrega mais 10 (decidido; sem N
    configurável).
  - A consulta no servidor passa a ter limite e paginação, para não mandar todo
    o histórico a cada atualização via SSE.

## Histórico em cartões (pedido em 2026-10-13)

- [x] **Histórico como sanfona por dia, com cartões e gaveta de detalhe.**
  Feito em 2026-10-14: `src/ui/history.rs`, `days_page`/`day_bois` em
  `src/views.rs`, rotas `room_history` (dias) e `room_day`.
  - Um item da sanfona por dia. Fechado, mostra o resumo do dia ("12/10 ·
    8 bois · André & Sobrinho 5 × 3 Pai & Tio"). Abertos: hoje e/ou o
    último dia jogado.
  - Dentro do dia, um cartão por boi com o resultado final (número, lados,
    placar, lambreta). Os cartões ficam em grade com
    `repeat(auto-fill, minmax(…, 1fr))`: o número de colunas se ajusta à
    largura da tela sem rolar para o lado.
  - Tocar no cartão abre uma gaveta de baixo para cima com o boi: as duas
    cruzes no estado final, dia, vencedor, quem saiu na primeira mão
    válida e a fileira de mãos (quadrados/retângulos, reaproveitando
    `HandStrip`). Botão de fechar grande, fecha também tocando fora.
  - Aprovado em 2026-10-13, com a paginação por dia e a gaveta com cruzes
    e resumo. Na fila depois de separar `src/model.rs`.
  - Proposta técnica: paginar por dia, não por boi. A página de 10 bois
    corta um dia no meio, o que não combina com a sanfona. O histórico
    traz o resumo dos dias, e os bois de um dia só são buscados quando ele
    abre. "Ver mais" passa a trazer mais dias.

## Estatísticas por período (pedido em 2026-10-12)

- [x] **Dados da sala por dia, mês e ano.** Feito em 2026-10-13: `Period` em `src/model.rs`, `api::room_stats`, `rules::firsts`
  e `Stats` em `src/ui/room.rs`. Um boi conta no dia em que começou. O
  seletor é Tudo / Dia / Mês / Ano, com os períodos que tiveram jogo.
  - [x] **Destaques do período:** quem fez melhor no dia, no mês, no ano
    (`rules::best` e `highlights` em `src/ui/room.rs`). Nas duplas, o
    melhor jogador só aparece quando não é a própria melhor dupla.
  - [x] **Primeiras do ano:** quem saiu primeiro em 2027, quem ganhou a 1ª mão, o
    1º boi, a 1ª lambreta e a 1ª lambreta reversa do ano (e quem levou).
  - [x] Os números que já existem (bois, mãos, batidas, fechados,
    lambretas), filtrados pelo período.
  - "Fez melhor" (decidido em 2026-10-13): mais bois ganhos; empate
    desempata por % de mãos ganhas. Mostrar junto quantos bois cada um
    jogou, porque com poucos jogos num dia isso varia muito.
  - Nas duplas, também mostrar a melhor dupla (decidido em 2026-10-13).
  - [x] **Comparar dois períodos** (pedido em 2026-10-13; ex.: março de 2026 ×
    março de 2027). O seletor ganha um "comparar com" opcional. Com ele
    ligado, a tabela mostra os dois períodos lado a lado por jogador, com a
    diferença (ex.: bois ganhos 5 → 8, +3). O endpoint recebe uma lista de
    períodos e devolve as estatísticas de cada um, então um período só é o
    caso de lista com um item. Decidido em 2026-10-13: só dois períodos,
    em tabela (mais que isso polui a tela). Primeiro escolhe se compara
    meses ou anos, depois os dois. Dia contra dia fica de fora. Na tela
    a comparação mostra só bois, mãos e lambretas ("3 → 8"): com mais
    colunas a tabela não cabia no celular.
  - A tabela simples (7 colunas) já precisava rolar para o lado no celular
    antes desta mudança. Vale rever junto com os destaques.
  - Notas técnicas: as datas seguem o fuso de Brasília (`format_date`). O
    cálculo vai para o servidor, num endpoint próprio, porque o histórico vai
    passar a carregar só 10 bois por vez. Mão tem `created_at` e boi tem
    `started_at`, então já dá para filtrar sem mudar o banco.

## Viradas (pedido em 2026-10-13)

- [x] **Durante o boi, mostrar quantas vezes quem estava atrás naquele
  placar virou.** Feito em 2026-10-13: `rules::comebacks`,
  `RoomView.comebacks` e `comeback_lines` em `src/ui/room.rs`. Inspirado nas estatísticas de esporte americano ("só X%
  dos times que perderam os 2 primeiros jogos levaram a série"). Precisa
  ser claro para um senhor de 82 anos que gosta de futebol.
  - Texto, sem porcentagem e sem gráfico, numa linha abaixo das cruzes:
    "Atrás por 1 a 3? Nesta sala, quem ficou assim virou 2 de 9 vezes."
    Palavras do futebol ("virou", "virada"). Contagem em vez de %, porque
    "2 de 9" já mostra quanta história existe por trás do número, e "22%"
    esconderia que foram só 9 vezes.
  - Os dados vêm da sala inteira, não só do confronto. É como no esporte:
    a estatística é de todos os times, não só daquele jogo. Com só o
    confronto, 5 bois dariam 1 ou 2 casos por placar.
  - Aparece quando a sala tiver pelo menos 5 bois terminados, alguém
    estiver atrás e aquele placar já tiver acontecido antes. Não aparece no
    0 a 0 nem com placar empatado.
  - Individual com 3 ou 4: o placar de cada um é comparado com o do líder
    ("2 a 3" é ter 2 com o líder em 3).
  - Mãos empatadas não mudam o placar, então não contam como caso novo.
  - Cálculo em `src/rules.rs` (puro, com testes), no servidor, como as
    estatísticas: varre os bois da sala e conta, para cada placar
    (atrás, líder), quantas vezes aconteceu e quantas o lado de trás levou.
  - Descartado: chance calculada pelas mãos do confronto (um modelo com
    porcentagem). É mais preciso com poucos dados, mas é difícil de
    explicar e de confiar para quem não é da área.
  - Depois, se valer a pena: destacar quando a virada acontecer de fato
    ("Virada! Só 2 tinham conseguido antes").
  - **Extras, não obrigatórios** (ideias do André em 2026-10-13):
    - **Empate em 2 a 2 ou 3 a 3.** Na sala inteira não diz nada (dá
      metade para cada lado por definição). Faz sentido só no confronto:
      "No 2 a 2 entre vocês: Pai levou 3, André 2". Em 1 a 1 não mostrar.
    - **Mão 7 (3 a 3, a decisiva): a saída pesa?** "Na mão decisiva, quem
      saiu levou 5 de 8 vezes." Leitura minha de "quantas pessoas
      confirmaram a saída"; confirmar com o André quando o item começar.
    - **Recuperações até o 3 a 3:** "Já aconteceu 4 vezes de alguém estar
      0 a 3 ou 1 a 3 e empatar em 3 a 3." Conta quem estava atrás por 2 ou
      3 e chegou ao empate, mesmo que tenha perdido a mão 7.

## Guia de uso (pedido em 2026-10-14, feito no mesmo dia)

Feito: `src/ui/help.rs` (textos por tela e convite), `src/ui/theme.rs`
(tema), `src/ui/top_bar.rs` (barra do topo de todas as telas). Na tela
inicial, o "Olá, nome · Minha conta · Sair" desceu para baixo do título,
porque com o tema e o "?" não cabia ao lado no celular.

Um passo a passo de como usar o app, **opcional**: não aparece à força no
primeiro uso e pode ser aberto de novo quando quiser.

- **Decidido em 2026-10-14: ajuda por tela, não um tour único.** Um botão
  "?" no topo, canto direito, com bom contraste nos dois temas, explica só
  a tela atual. Quem volta com uma dúvida ("como desfaço?") acha a resposta
  sem refazer o tour inteiro.
- **Ao lado do "?", a troca de tema**: claro, escuro ou automático (hoje o
  app só segue o sistema, por `prefers-color-scheme` em `assets/main.css`).
  A escolha fica no `localStorage` do aparelho.
- Na primeira visita, no máximo um convite discreto apontando para o "?",
  que some ao fechar. Nada de tela cheia bloqueando o uso.
- Textos curtos, com o vocabulário do jogo: abrir ou criar a sala, marcar a
  mão (batida, fechado, empate), ler a cruz e o placar, desfazer, histórico
  e estatísticas. Pensar no senhor de 82 anos: letra grande, botão de
  fechar grande.
- "Já vi o convite" e o tema escolhido ficam no `localStorage` (conveniência;
  se sumir, volta o padrão).

## Infra

- [x] "Bug" do `onsubmit` no Safari: era o teste clicando antes da hidratação.
- [x] Teste da interface no Safari via WebDriver em `tools/ui_test.py`.
- [x] **Botões apagados até o app carregar** (regra no fim dos botões em
  `assets/main.css`). Cliques antes da
  hidratação se perdem, num celular lento. CSS: enquanto `<html>` não tiver
  `data-pronto`, botões, links de ação e campos ficam apagados e sem clique.
- [x] Deploy: Turso e hospedagem. Escolhido e feito em 2026-10-14: Cloud Run em São
  Paulo + Turso em São Paulo. Motivos e passos em `docs/DEPLOY.md`.
- [x] **CI no GitHub Actions** (pedido em 2026-10-13). Feito:
  `.github/workflows/ci.yml`. A cada push na `main` e em PR:
  - `cargo fmt --check`.
  - `cargo clippy` nas duas pontas (`--features server` e
    `--target wasm32-unknown-unknown`) com `-D warnings`.
  - `cargo test --features server`.
  - Cache do `target/` (`Swatinem/rust-cache`), senão cada execução leva
    minutos.
  - Fica de fora por enquanto: `tools/ui_test.py`. Ele usa o Safari, e no
    GitHub isso exige runner macOS (mais lento e caro em minutos) e instalar
    o `dx`. Vale reavaliar se o teste de interface passar a pegar bugs que o
    `cargo test` não pega. `tools/api_test.sh` só imprime, não verifica nada;
    para entrar no CI teria que virar asserções.
- [x] **Recuperar senha** (pedido em 2026-10-13). Feito em 2026-10-13 com
  e-mail: "Minha conta" confirma o e-mail por código; "Entrar com código" e
  "Esqueci a senha" mandam um código de 6 dígitos (15 min, 5 tentativas,
  um por minuto); o administrador (`PLACAR_ADMIN`) gera link de 1 hora e
  manda pelo próprio WhatsApp com a mensagem pronta (wa.me, sem
  automação). Trocar a senha encerra as outras sessões.
  - [ ] **Envio de e-mail de verdade.** Hoje `send_email` em
    `src/accounts.rs` só escreve no log (e em `PLACAR_MAILBOX`). Precisa de
    domínio próprio verificado num serviço (ex.: Resend). Fazer no deploy.
  - Histórico da decisão: Hoje a conta só tem
  usuário e senha, sem e-mail, então não há para onde mandar um link.
  Proposta para agora: o dono do app gera um link de troca de senha de uso
  único, que vale por 1 hora, e manda pelo WhatsApp. Só guardar o hash do
  token. Por e-mail fica para quando houver deploy e domínio: pede campo de
  e-mail, serviço de envio (ex.: Resend) e verificação do endereço.
  Proposta aceita em 2026-10-13. Quem gera: só o André, por enquanto (o
  jeito exato, comando no servidor ou página de administrador, fica para
  quando o item começar).
  - **(em discussão, 2026-10-13)** O André apontou o furo: se quem esquece
    a senha é ele, a página de administrador não serve (não dá para
    entrar). Proposta nova: e-mail opcional na conta; "Esqueci a senha"
    manda um código de 6 dígitos (vale 15 min, 5 tentativas), e o mesmo
    mecanismo permite "Entrar com código" como opção ao lado da senha.
    Envio por um serviço de e-mail (ex.: Resend, plano grátis), que exige
    domínio próprio verificado, ou seja, depende do deploy. Dá para fazer
    tudo antes com um "carteiro" de desenvolvimento que só escreve o código
    no log do servidor, e ligar o envio real depois. O link gerado pelo
    administrador fica como reserva para quem não tem e-mail.
    WhatsApp e SMS descartados por agora: a API oficial do WhatsApp pede
    verificação da empresa na Meta e modelos aprovados; SMS é pago por
    mensagem e pede cadastro de remetente.
- [x] **Comprimir a interface** (feito em 2026-10-14): `pre_compress = true`
  no `Dioxus.toml`. O `dx bundle` grava um `.br` ao lado de cada arquivo e o
  servidor do Dioxus entrega o `.br` a quem aceita Brotli (o Safari só pede
  em HTTPS). O wasm caiu de 3,0 MB para 647 KB em produção. Ficaram sem
  compressão a página (HTML do SSR) e as respostas da API, que são pequenas.
- [x] **CD** (feito em 2026-10-14, ver `docs/DEPLOY.md`). Compilar no GitHub Actions com cache e só
  mandar a imagem pronta ao Cloud Run (no Cloud Build cada deploy leva
  ~15 min, compilando tudo do zero). Não dá para automatizar o deploy antes de
  escolher a hospedagem. Quando houver: push na `main` com CI verde publica
  o servidor. Para o APK (ver "Futuro: distribuição"): tag `v*` gera o APK
  assinado e anexa ao Release, com a chave guardada nos secrets do GitHub.
- [ ] Fase 2: regras configuráveis.

## Arquitetura

- **Refatorar para DDD? Avaliado em 2026-10-13: não, como doutrina.** O
  código tem 4 mil linhas, um desenvolvedor e um domínio pequeno. Os
  padrões táticos (agregados, repositórios, serviços de domínio, eventos)
  trariam mais arquivos e camadas do que clareza. O que o DDD tem de mais
  útil o projeto já tem: a linguagem do domínio no código (boi, mão,
  lambreta, saída, fechado) e um núcleo de regras puro e testado
  (`src/rules.rs`, sem banco e sem tela). Três ajustes pontuais valem a
  pena, e entraram na "Ordem atual" (itens 10 a 12) a pedido do André:
  - [x] **Tirar regras dos handlers de `src/api.rs`.** Feito em
    2026-10-13: `src/room_rules.rs` (regras da sala, separadas das regras
    do jogo em `rules.rs`), com o tipo `Refusal` virando a resposta HTTP. Hoje "pode remover
    este jogador?", "o boi atual acabou?" e "o que o desfazer apaga?" estão
    misturados com SQL e não têm teste. Viram funções puras em
    `rules.rs`, com teste, e o handler só carrega, chama e grava.
  - [x] **Separar `src/model.rs`** (feito em 2026-10-13: `model.rs` com o
    domínio, `period.rs` com datas e períodos, `views.rs` com o que viaja
    entre servidor e tela): tipos do domínio e funções (`Period`,
    cores das duplas, paginação) num lugar, tipos que só viajam entre
    servidor e tela (`RoomView`, `PeriodReport`, `HistoryPage`) em outro.
  - [x] **Contas separadas do placar** no servidor: `src/accounts.rs` e
    `src/api/accounts.rs`.

## Futuro: distribuição

Objetivo do André: aprender a lançar um app numa loja. Ordem: primeiro o APK
no GitHub (Releases do repositório público `dede999/family-domino-score`),
depois a Play Store.

- [ ] **Deploy do servidor antes de qualquer APK.** O app Android é só a
  interface: cadastro, salas e placar ao vivo vêm do servidor. Sem servidor
  publicado (Turso + hospedagem), o APK não funciona fora de casa.
- [ ] **Ambiente Android no KINGSTON.** SDK e NDK somam vários GB, e o disco
  interno tem menos de 1 GB livre. Instalar com `ANDROID_HOME` no KINGSTON.
  Build com `dx bundle --platform android`.
- [ ] **APK em Releases do GitHub.** A família instala por fora da loja
  ("fontes desconhecidas"). Assinar sempre com a mesma chave, guardada fora
  do repositório: sem ela não dá para publicar atualizações.
- [ ] **Play Store.** Pontos a conferir na época, porque as regras mudam:
  - A taxa de conta de desenvolvedor do Google é paga uma vez (US$ 25 na
    última vez que conferi), não por ano. A anuidade de US$ 99 é da Apple.
  - Contas pessoais novas precisam de um teste fechado com pelo menos 12
    testadores por 14 dias seguidos antes de publicar em produção. A família
    pode ser esse grupo.
  - Verificação de identidade com documento.
  - A loja pede AAB (não APK), um nível mínimo recente de API do Android e
    uma página de política de privacidade.
  - App com cadastro precisa oferecer **exclusão de conta** dentro do app e
    por um link na web. Hoje o app não tem isso.
