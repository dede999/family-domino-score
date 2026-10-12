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
5. Mãos do boi como fileira de quadrados (próximo), depois o histórico.

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

- [ ] **Mãos do boi como fileira de quadrados, no topo e em destaque.**
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

- [ ] **Mostrar só os últimos N bois no histórico.**
  - Mostra 10 e um botão "ver mais" carrega mais 10 (decidido; sem N
    configurável).
  - A consulta no servidor passa a ter limite e paginação, para não mandar todo
    o histórico a cada atualização via SSE.

## Infra

- [x] "Bug" do `onsubmit` no Safari: era o teste clicando antes da hidratação.
- [x] Teste da interface no Safari via WebDriver em `tools/ui_test.py`.
- [x] **Botões apagados até o app carregar** (regra no fim dos botões em
  `assets/main.css`). Cliques antes da
  hidratação se perdem, num celular lento. CSS: enquanto `<html>` não tiver
  `data-pronto`, botões, links de ação e campos ficam apagados e sem clique.
- [ ] Deploy: Turso e hospedagem.
- [ ] Fase 2: regras configuráveis.

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
