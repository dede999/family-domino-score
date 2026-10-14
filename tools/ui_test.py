"""Teste da interface no Safari via WebDriver, sem dependências."""
import base64, json, os, random, subprocess, sys, time, urllib.request

WD = "http://127.0.0.1:4444"
APP = "http://127.0.0.1:8080"
# Onde o servidor de teste escreve os e-mails (variável PLACAR_MAILBOX do dx).
MAILBOX = os.environ.get("PLACAR_MAILBOX", "/Volumes/KINGSTON/.cargo-build/tmp/mailbox.txt")
OUT = os.path.dirname(os.path.abspath(__file__))
sid = None


def call(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(WD + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return json.load(r)["value"]
    except urllib.error.HTTPError as e:
        return {"error": json.load(e)["value"]}


def s(method, path, body=None):
    return call(method, f"/session/{sid}{path}", body)


def find(xpath, timeout=10):
    end = time.time() + timeout
    while time.time() < end:
        v = s("POST", "/element", {"using": "xpath", "value": xpath})
        if "error" not in v:
            return list(v.values())[0]
        time.sleep(0.3)
    shot("falha")
    raise SystemExit(f"FALHOU: não achei {xpath}\nTela:\n{body_text()[:1500]}")


# Conta os cliques que chegam à página. Quando a janela do Safari
# automatizado perde o foco (alguém usando outra janela do Mac), o WebDriver
# responde que clicou e a página não recebe nada. Nesse caso o clique é
# repetido pelo JavaScript, com aviso.
COUNT_CLICKS = """
if (window.__clicks === undefined) {
  window.__clicks = 0;
  document.addEventListener('click', () => window.__clicks++, true);
}
return window.__clicks;
"""

JS_CLICK = """
const e = document.evaluate(arguments[0], document, null, 9, null).singleNodeValue;
if (!e) return false;
e.click();
return true;
"""


def click(xpath):
    el = find(xpath)
    before = s("POST", "/execute/sync", {"script": COUNT_CLICKS, "args": []})
    s("POST", f"/element/{el}/click", {})
    time.sleep(0.4)
    after = s("POST", "/execute/sync", {"script": "return window.__clicks", "args": []})
    # Se a página recarregou, o contador sumiu (None) e o clique valeu.
    if after == before:
        focus = s("POST", "/execute/sync", {"script": "return document.hasFocus()", "args": []})
        print(f"AVISO clique não chegou (foco na janela: {focus}); repetindo por JS: {xpath}", flush=True)
        s("POST", "/execute/sync", {"script": JS_CLICK, "args": [xpath]})
        time.sleep(0.4)


def type_in(xpath, text):
    el = find(xpath)
    s("POST", f"/element/{el}/clear", {})
    s("POST", f"/element/{el}/value", {"text": text})


def body_text():
    v = s("POST", "/execute/sync", {"script": "return document.body.innerText", "args": []})
    return v if isinstance(v, str) else str(v)


def wait_text(text, timeout=10):
    end = time.time() + timeout
    while time.time() < end:
        if text in body_text():
            return
        time.sleep(0.3)
    shot("falha")
    raise SystemExit(f"FALHOU: texto '{text}' não apareceu\nTela:\n{body_text()[:1500]}")


def shot(name):
    png = s("GET", "/screenshot")
    if isinstance(png, str):
        with open(os.path.join(OUT, f"{name}.png"), "wb") as f:
            f.write(base64.b64decode(png))


def wait_ready(timeout=30):
    """Espera o app hidratar (marca posta em `App`). Antes disso os cliques se perdem."""
    end = time.time() + timeout
    while time.time() < end:
        v = s("POST", "/execute/sync", {"script": "return document.documentElement.dataset.pronto || ''", "args": []})
        if v == "1":
            return
        time.sleep(0.2)
    shot("falha")
    raise SystemExit("FALHOU: o app não hidratou")


def last_code(to, timeout=10):
    """Último código de 6 dígitos mandado para `to` (lido da caixa de teste)."""
    end = time.time() + timeout
    while time.time() < end:
        try:
            lines = [l for l in open(MAILBOX, encoding="utf-8") if l.startswith(f"para {to} |")]
        except FileNotFoundError:
            lines = []
        if lines:
            digits = [w.strip(".") for w in lines[-1].split() if w.strip(".").isdigit() and len(w.strip(".")) == 6]
            if digits:
                return digits[-1]
        time.sleep(0.3)
    raise SystemExit(f"FALHOU: nenhum código para {to} em {MAILBOX}")


def btn(text):
    return f"//button[normalize-space()='{text}']"


def ok(msg):
    print("OK  ", msg, flush=True)


driver = subprocess.Popen(["safaridriver", "-p", "4444"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(20):
        try:
            urllib.request.urlopen(WD + "/status", timeout=1)
            break
        except Exception:
            time.sleep(0.3)
    v = call("POST", "/session", {"capabilities": {"alwaysMatch": {"browserName": "safari"}}})
    if "error" in v:
        raise SystemExit(f"Sessão não criada: {v}")
    sid = v["sessionId"]
    s("POST", "/window/rect", {"width": 420, "height": 900})

    # 1. Página inicial e cadastro
    s("POST", "/url", {"url": APP})
    wait_ready()
    wait_text("Acompanhar uma sala")
    click("//button[normalize-space()='Criar uma conta']")
    user = f"teste{random.randint(1000, 9999)}"
    type_in("//label[contains(.,'Usuário')]/input", user)
    type_in("//label[contains(.,'Seu nome')]/input", "André")
    type_in("//label[contains(.,'Senha')]/input", "segredo1")
    click("//form[contains(@class,'stack')]//button[@type='submit']")
    wait_text("Olá, André")
    ok(f"cadastro e login ({user})")

    # 1b. Minha conta: confirma um e-mail com o código
    email = f"{user}@exemplo.com"
    click("//a[normalize-space()='Minha conta']")
    wait_text("Adicionar e-mail")
    type_in("//label[contains(.,'E-mail')]/input", email)
    click(btn("Mandar código"))
    wait_text("Mandamos um código")
    type_in("//label[contains(.,'Código')]/input", last_code(email))
    click(btn("Confirmar e-mail"))
    wait_text(email)
    wait_text("Trocar e-mail")
    shot("1b-minha-conta")
    click("//a[normalize-space()='← Início']")
    wait_text("Minhas salas")
    ok("e-mail confirmado com código")

    # 2. Erro amigável: criar sala sem nome
    click("//a[normalize-space()='Nova sala']")
    wait_text("Modo de jogo")
    time.sleep(0.5)
    shot("1-nova-sala")
    click(btn("Criar sala"))
    wait_text("Dê um nome para a sala.")
    assert "error running server function" not in body_text(), "mensagem de erro com prefixo técnico"
    ok("mensagem de erro amigável")

    # 3. Cria sala de duplas
    type_in("//label[contains(.,'Nome da sala')]/input", "Dominó de casa")
    names = ["André", "Pai", "Sobrinho", "Tio"]
    rows = "(//div[@class='player-row'])"
    for i, n in enumerate(names, 1):
        type_in(f"{rows}[{i}]/input[1]", n)
    click(btn("Criar sala"))
    wait_text("Nenhum boi ainda.")
    code = find("//strong[@class='code']")
    code = s("GET", f"/element/{code}/text")
    ok(f"sala criada ({code})")

    # 3b. Cores das duplas: André & Sobrinho troca a principal para Roxo
    wait_text("Cores das duplas")
    click("(//div[@class='pair-colors'])[1]//button[normalize-space()='Mudar']")
    click("(//div[@class='pair-colors'])[1]//fieldset[legend='Principal']//button[@title='Roxo']")
    find("(//div[@class='pair-colors'])[1]//span[@title='Roxo']")
    shot("2b-cores-duplas")
    click("(//div[@class='pair-colors'])[1]//button[normalize-space()='Pronto']")
    ok("cor da dupla trocada")

    # 4. Boi: regra da carroça, empate, depois saída girando
    click(btn("Começar boi"))
    wait_text("Quem saiu?")
    wait_text("maior carroça")
    shot("2-quem-saiu")
    click(btn("Pai"))
    click(btn("Fechado empatado"))
    wait_text("maior carroça")  # empate: continua perguntando
    ok("empate mantém a regra da carroça")
    click(btn("Sobrinho"))
    # Dupla B ganha uma (Pai & Tio)
    click("(//div[@class='win-group'])[2]/button[normalize-space()='Bateu']")
    wait_text("Sai: Tio")
    ok("depois da primeira vitória, a saída gira (Sobrinho → Tio)")
    # Dupla A faz 4 seguidas: lambreta reversa
    for i in range(4):
        click("(//div[@class='win-group'])[1]/button[normalize-space()='Ganhou no fechado']" if i == 0
              else "(//div[@class='win-group'])[1]/button[normalize-space()='Bateu']")
        time.sleep(0.6)
    wait_text("ganhou o boi!")
    wait_text("Lambreta reversa em Pai & Tio!")
    shot("3-fim-do-boi")
    ok("fim do boi com lambreta reversa")
    assert len(s("POST", "/elements", {"using": "xpath", "value": "//div[@class='hand-strip']/button"})) == 6
    click("//div[@class='hand-strip']/button[1]")
    wait_text("Mão 1: Pai saiu, fechado empatado")
    shot("3b-detalhe-mao")
    ok("fileira com as 6 mãos e detalhe ao tocar")

    # 5. Desfazer volta o boi
    click(btn("Desfazer última mão"))
    wait_text("Sai: ")
    assert "ganhou o boi!" not in body_text()
    click("(//div[@class='win-group'])[1]/button[normalize-space()='Bateu']")
    wait_text("ganhou o boi!")
    ok("desfazer e refazer")

    # 6. Tempo real: outra pessoa (via API) começa um boi novo e marca uma mão
    jar = os.path.join(OUT, "jar_ui.txt")
    subprocess.run(["curl", "-s", "-c", jar, "-X", "POST", "-H", "Content-Type: application/json",
                    f"{APP}/api/entrar", "-d", json.dumps({"username": user, "password": "segredo1"})],
                   capture_output=True)
    subprocess.run(["curl", "-s", "-b", jar, "-X", "POST", "-H", "Content-Type: application/json",
                    f"{APP}/api/salas/{code}/boi", "-d", "{}"], capture_output=True)
    wait_text("Boi nº 2", timeout=8)
    subprocess.run(["curl", "-s", "-b", jar, "-X", "POST", "-H", "Content-Type: application/json",
                    f"{APP}/api/salas/{code}/maos", "-d",
                    json.dumps({"expected_hands": 0, "starter_seat": 0, "outcome": "Batida", "winner_side": 1})],
                   capture_output=True)
    find("//div[@class='hand-strip']/button[1]", timeout=8)
    ok("placar atualizou sozinho (tempo real)")

    # 7. Estatísticas e histórico
    t = body_text()
    assert "Estatísticas da sala" in t and "Histórico" in t, "faltam estatísticas/histórico"
    shot("4-estatisticas")
    ok("estatísticas e histórico aparecem")

    # 7b. Histórico em cartões: termina o boi 2 e joga mais 10 (12 terminados)
    def api(path, body):
        subprocess.run(["curl", "-s", "-b", jar, "-X", "POST", "-H", "Content-Type: application/json",
                        f"{APP}/api/salas/{code}/{path}", "-d", json.dumps(body)], capture_output=True)
    for n in range(1, 4):
        api("maos", {"expected_hands": n, "starter_seat": 0, "outcome": "Batida", "winner_side": 1})
    for _ in range(10):
        api("boi", {})
        for n in range(4):
            api("maos", {"expected_hands": n, "starter_seat": 0, "outcome": "Batida", "winner_side": 0})
    cards = "//div[@class='boi-grid']/button"
    end = time.time() + 8
    while len(s("POST", "/elements", {"using": "xpath", "value": cards})) != 12 and time.time() < end:
        time.sleep(0.3)
    assert len(s("POST", "/elements", {"using": "xpath", "value": cards})) == 12, "hoje não mostra os 12 bois"
    wait_text("12 bois · André & Sobrinho 11 × 1 Pai & Tio")
    click(f"{cards}[normalize-space(.//span[@class='boi-num'])='Boi 1']")
    wait_text("Saiu na primeira mão válida")
    find("//div[@class='drawer']//h2[.='Boi nº 1']")
    shot("4b-gaveta")
    click("//div[@class='drawer']//button[.='Fechar']")
    end = time.time() + 5
    while s("POST", "/elements", {"using": "xpath", "value": "//div[@class='drawer']"}) and time.time() < end:
        time.sleep(0.2)
    assert not s("POST", "/elements", {"using": "xpath", "value": "//div[@class='drawer']"}), "gaveta não fechou"
    ok("histórico de hoje em cartões, com gaveta")

    # 7c. Estatísticas por período (todos os bois são de hoje)
    click(btn("Ano"))
    find("//select[@aria-label='Período']/option[.='2026']")
    wait_text("11/12")
    wait_text("Destaque de 2026")
    wait_text("André & Sobrinho: 11 de 12 bois, 88% das mãos")
    # A dupla não mudou, então o melhor jogador é ela e não se repete.
    assert "Jogador: " not in body_text(), "destaque de jogador repetindo a dupla"
    wait_text("Primeiras de 2026")
    assert "Pai saiu" not in body_text()
    wait_text("André & Sobrinho em Pai & Tio (boi 1,")
    shot("4c-primeiras-do-ano")
    click(btn("Dia"))
    find("//select[@aria-label='Período']/option[contains(., '/2026')]")
    wait_text("11/12")
    ok("estatísticas por ano (destaque e primeiras) e por dia")

    # 7d. Viradas: boi 13, Pai & Tio abrem 1 a 0
    api("boi", {})
    api("maos", {"expected_hands": 0, "starter_seat": 0, "outcome": "Batida", "winner_side": 1})
    wait_text("André & Sobrinho atrás por 0 a 1?", timeout=8)
    s("POST", "/execute/sync", {"script": "window.scrollTo(0, 0)", "args": []})
    shot("4d-viradas")
    ok("viradas aparecem para quem está atrás")

    # 7e. Histórico por dia: leva os bois 1 a 6 para os 6 dias anteriores
    # direto no banco de teste (o app não deixa mudar a data de um boi).
    db = os.environ.get("PLACAR_DB", "/Volumes/KINGSTON/.cargo-build/tmp/teste.db")
    subprocess.run(["sqlite3", db, f"""
        UPDATE bois SET started_at = started_at - 86400 * (7 - (
            SELECT COUNT(*) FROM bois b2 WHERE b2.room_id = bois.room_id AND b2.id <= bois.id))
        WHERE room_id = (SELECT id FROM rooms WHERE code = '{code}')
          AND (SELECT COUNT(*) FROM bois b2 WHERE b2.room_id = bois.room_id AND b2.id <= bois.id) <= 6
    """], check=True)
    s("POST", "/refresh", {})
    wait_ready()
    heads = "//button[@class='day-head']"
    wait_text("Ontem")
    assert len(s("POST", "/elements", {"using": "xpath", "value": heads})) == 5, "primeira página não tem 5 dias"
    click(btn("Ver mais dias"))
    end = time.time() + 5
    while len(s("POST", "/elements", {"using": "xpath", "value": heads})) != 7 and time.time() < end:
        time.sleep(0.2)
    assert len(s("POST", "/elements", {"using": "xpath", "value": heads})) == 7, "Ver mais dias não trouxe 7"
    assert not s("POST", "/elements", {"using": "xpath", "value": btn("Ver mais dias")}), "sobrou o Ver mais dias"
    # Dias fechados só buscam os bois quando abrem.
    click(f"{heads}[contains(., 'Ontem')]")
    wait_text("Boi 6")
    shot("4e-dias")
    ok("histórico por dia: 5 dias, Ver mais dias e dia fechado abre")

    # 8. Visitante: sai da conta e abre a sala pelo código
    s("POST", "/url", {"url": APP})
    wait_ready()
    click(btn("Sair"))
    wait_text("Criar uma conta")
    type_in("//input[@placeholder='Código da sala']", code.lower())
    click(btn("Abrir"))
    wait_text("Você está assistindo")
    assert "Bateu" not in body_text(), "visitante vê botões de marcar"
    shot("5-visitante")
    ok("visitante vê o placar sem botões")

    # 8b. Ajuda da tela e tema, no topo
    click(btn("?"))
    wait_text("Como usar a sala")
    assert "Desfazer última mão" in body_text(), "ajuda da sala sem o desfazer"
    shot("5b-ajuda")
    click(btn("Fechar"))
    end = time.time() + 5
    while "Como usar a sala" in body_text() and time.time() < end:
        time.sleep(0.2)
    assert "Como usar a sala" not in body_text(), "ajuda não fechou"
    theme = lambda: s("POST", "/execute/sync", {"script": "return document.documentElement.dataset.theme", "args": []})
    seen = set()
    for _ in range(3):  # automático → claro → escuro → automático
        click("//button[contains(@class,'theme')]")
        seen.add(theme())
    assert seen == {"light", "dark"}, f"tema não alternou: {seen}"
    ok("ajuda da tela abre e fecha; tema alterna")

    # 9. Entrar com código e esqueci a senha
    s("POST", "/url", {"url": APP})
    wait_ready()
    click("//a[normalize-space()='Entrar com código']")
    type_in("//label[contains(.,'Usuário ou e-mail')]/input", email)
    click(btn("Mandar código"))
    wait_text("o código chegou lá")
    type_in("//label[contains(.,'Código')]/input", last_code(email))
    click(btn("Entrar"))
    wait_text("Olá, André")
    ok("entrou com código")
    click(btn("Sair"))
    wait_text("Esqueci a senha")
    before = last_code(email)
    # O servidor só manda um código por minuto para a mesma conta.
    time.sleep(61)
    click("//a[normalize-space()='Esqueci a senha']")
    type_in("//label[contains(.,'Usuário ou e-mail')]/input", user)
    click(btn("Mandar código"))
    wait_text("o código chegou lá")
    end = time.time() + 10
    while (new := last_code(email)) == before and time.time() < end:
        time.sleep(0.3)
    type_in("//label[contains(.,'Código')]/input", new)
    type_in("//label[contains(.,'Senha nova')]/input", "outrasenha")
    click(btn("Trocar a senha e entrar"))
    wait_text("Olá, André")
    ok("trocou a senha esquecida pelo código")
    print("TUDO OK")
finally:
    if sid:
        call("DELETE", f"/session/{sid}")
    driver.terminate()
