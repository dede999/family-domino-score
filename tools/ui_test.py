"""Teste da interface no Safari via WebDriver, sem dependências."""
import base64, json, os, random, subprocess, sys, time, urllib.request

WD = "http://127.0.0.1:4444"
APP = "http://127.0.0.1:8080"
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


def click(xpath):
    s("POST", f"/element/{find(xpath)}/click", {})
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

    # 7b. Histórico paginado: termina o boi 2 e joga mais 10 (12 terminados)
    def api(path, body):
        subprocess.run(["curl", "-s", "-b", jar, "-X", "POST", "-H", "Content-Type: application/json",
                        f"{APP}/api/salas/{code}/{path}", "-d", json.dumps(body)], capture_output=True)
    for n in range(1, 4):
        api("maos", {"expected_hands": n, "starter_seat": 0, "outcome": "Batida", "winner_side": 1})
    for _ in range(10):
        api("boi", {})
        for n in range(4):
            api("maos", {"expected_hands": n, "starter_seat": 0, "outcome": "Batida", "winner_side": 0})
    wait_text("Boi 12:", timeout=8)
    hist = "//section[h2='Histórico']//li"
    assert len(s("POST", "/elements", {"using": "xpath", "value": hist})) == 10, "histórico não mostra 10"
    click(btn("Ver mais"))
    wait_text("Boi 1:")
    assert len(s("POST", "/elements", {"using": "xpath", "value": hist})) == 12
    assert not s("POST", "/elements", {"using": "xpath", "value": btn("Ver mais")}), "sobrou o Ver mais"
    shot("4b-ver-mais")
    ok("histórico mostra 10 e o Ver mais traz o resto")

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
    print("TUDO OK")
finally:
    if sid:
        call("DELETE", f"/session/{sid}")
    driver.terminate()
