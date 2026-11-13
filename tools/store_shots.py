"""Capturas de tela para a Play Store (1080x1920), com uma sala de exemplo.

Uso, com o servidor local rodando num banco vazio só para isso:
    PLACAR_DB=/Volumes/KINGSTON/.cargo-build/tmp/loja.db dx serve ...   (ver docs/ESTADO.md)
    python3 tools/store_shots.py /Volumes/KINGSTON/.cargo-build/tmp/loja.db
    python3 tools/store_shots.py BANCO CODIGO   (só as fotos, com a sala já criada)

Cria a conta "andre" e a sala "Dominó de domingo" com bois de alguns dias,
volta as datas no banco e tira as fotos de celular, tablet e computador em
dist/loja/, pelo Chromium do Playwright (`pip install playwright`).
"""
import http.cookiejar, json, os, random, subprocess, sys, time, urllib.request

from PIL import Image

APP = "http://127.0.0.1:8080"
DB = sys.argv[1]
OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "dist", "loja")
os.makedirs(OUT, exist_ok=True)

jar = http.cookiejar.CookieJar()
web = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))


def api(path, body=None):
    data = json.dumps(body if body is not None else {}).encode()
    req = urllib.request.Request(APP + path, data=data, headers={"Content-Type": "application/json"})
    try:
        with web.open(req, timeout=30) as r:
            raw = r.read()
            return True, json.loads(raw) if raw else None
    except urllib.error.HTTPError as e:
        return False, e.read().decode()


def seed():
    """Conta, sala e bois de exemplo; devolve o código da sala."""
    global code
    # 1. Conta e sala. Duplas: assentos 0 e 2 (André e Tio Zé) contra 1 e 3.
    ok, msg = api("/api/cadastro", {"username": "andre", "display_name": "André", "password": "exemplo123"})
    assert ok, msg
    players = [{"id": None, "name": n, "username": u, "color": None}
               for n, u in (("André", "andre"), ("Pai", None), ("Tio Zé", None), ("Marcos", None))]
    ok, code = api("/api/salas", {"name": "Dominó de domingo", "mode": "Duplas", "players": players})
    assert ok, code


    def hand(n, outcome, side):
        for seat in range(4):
            ok, _ = api(f"/api/salas/{code}/maos", {"expected_hands": n, "starter_seat": seat,
                                                     "outcome": outcome, "winner_side": side})
            if ok:
                return True
        return False


    rnd = random.Random(7)
    BOIS = 17
    for b in range(BOIS):
        ok, msg = api(f"/api/salas/{code}/boi")
        assert ok, msg
        arms, n = [0, 0], 0
        last = b == BOIS - 1
        while max(arms) < 4:
            if last and arms == [2, 1]:
                break  # o último boi fica no meio, para a foto do placar
            r = rnd.random()
            if r < 0.08:
                outcome, side = "Empate", None
            else:
                side = 0 if rnd.random() < 0.55 else 1
                outcome = "Fechado" if rnd.random() < 0.15 else "Batida"
            if not hand(n, outcome, side):
                break
            n += 1
            if side is not None:
                arms[side] += 1

    # 2. Espalha os bois por alguns dias: os 6 últimos hoje, os outros antes.
    DAY = 86400
    plan = {}
    older = BOIS - 6
    for i in range(older):
        plan[i + 1] = 1 + (older - 1 - i) // 4  # 4 bois por dia; os primeiros, mais atrás
    sql = []
    for boi_seq, days in plan.items():
        delta = days * DAY
        sql.append(f"UPDATE maos SET created_at = created_at - {delta} WHERE boi_id = "
                   f"(SELECT id FROM bois ORDER BY id LIMIT 1 OFFSET {boi_seq - 1});")
        sql.append(f"UPDATE bois SET started_at = started_at - {delta} WHERE id = "
                   f"(SELECT id FROM bois ORDER BY id LIMIT 1 OFFSET {boi_seq - 1});")
    subprocess.run(["sqlite3", DB], input="\n".join(sql), text=True, check=True)
    return code


code = sys.argv[2] if len(sys.argv) > 2 else seed()


# 3. Fotos no Chromium do Playwright, no tamanho exato de cada aparelho.
# A loja aceita 9:16 ou 16:9; tablet de 7" e de 10" usam as mesmas fotos.
from playwright.sync_api import sync_playwright

DEVICES = [
    # nome, largura e altura da página (pontos), densidade da tela
    ("celular", 360, 640, 3),      # 1080x1920
    ("tablet", 720, 1280, 2),      # 1440x2560
    ("computador", 1280, 720, 1.5),  # 1920x1080 (Chromebook)
]


def take(browser, name, w, h, scale):
    ctx = browser.new_context(viewport={"width": w, "height": h}, device_scale_factor=scale,
                              locale="pt-BR", is_mobile=name == "celular", has_touch=name != "computador")
    ctx.request.post(APP + "/api/entrar", data={"username": "andre", "password": "exemplo123"})
    page = ctx.new_page()
    page.add_init_script("try { localStorage.setItem('convite-ajuda', '1'); } catch (e) {}")

    def go(path, text):
        page.goto(APP + path)
        page.wait_for_selector("html[data-pronto='1']")
        page.get_by_text(text).first.wait_for()
        page.wait_for_timeout(1500)

    def top_at(text):
        page.get_by_text(text, exact=True).first.evaluate(
            "el => { el.scrollIntoView({block: 'start'}); window.scrollBy(0, -12); }")
        page.wait_for_timeout(800)

    last = [None]

    def shot(n):
        png = page.screenshot()
        if png == last[0]:
            return  # a página acabou antes: a foto repetiria a anterior
        last[0] = png
        path = os.path.join(OUT, f"{name}-{n}.png")
        with open(path, "wb") as f:
            f.write(png)
        print("foto", os.path.basename(path), Image.open(path).size)

    go(f"/sala/{code}", "Boi nº")
    shot("1-placar")
    top_at("Histórico")
    shot("2-historico")
    top_at("Estatísticas da sala")
    shot("3-estatisticas")
    go("/", "Minhas salas")
    shot("4-inicio")
    ctx.close()


with sync_playwright() as pw:
    browser = pw.chromium.launch()
    for device in DEVICES:
        take(browser, *device)
    browser.close()
print("sala", code)
