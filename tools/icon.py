"""Gera o ícone do app e as imagens da Play Store a partir de um desenho só.

Uso: python3 tools/icon.py   (precisa do Pillow)

O desenho é uma pedra de dominó (3 e 5) inclinada sobre o verde do app
(`--accent` em `assets/main.css`). Saem, em `icons/`:
- `icon.svg`: o desenho em vetor, para usar onde precisar;
- `play-icon-512.png`: ícone da loja (512x512, quadrado; a loja arredonda);
- `play-feature-1024x500.png`: imagem de destaque da loja;
- `android/`: ícones do launcher, por densidade, mais o ícone adaptativo
  (fundo e frente separados), que o `Dioxus.toml` não gera sozinho.
"""
import math, os
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "icons")

GREEN = "#1d6b4f"
TILE = "#fffdf8"
PIP = "#1f1d1a"
LINE = "#d9d3c7"
GOLD = "#b8860b"

# Pedra em coordenadas de 0 a 1, centrada, antes de girar.
TILE_W, TILE_H, TILE_R = 0.31, 0.58, 0.05
ANGLE = -18  # graus; negativo = topo para a direita
PIP_R = 0.034
# Pontos de cada metade, em frações da metade (0..1 na largura e na altura).
THREE = [(0.25, 0.25), (0.5, 0.5), (0.75, 0.75)]
FIVE = [(0.25, 0.25), (0.75, 0.25), (0.5, 0.5), (0.25, 0.75), (0.75, 0.75)]


def pips():
    """Centros dos pontos, relativos ao centro da pedra."""
    out = []
    half = TILE_H / 2
    inset = 0.035
    side = TILE_W - 2 * inset  # cada metade é um quadrado
    for y0, spots in ((-half + inset, THREE), (inset, FIVE)):
        for fx, fy in spots:
            out.append((-TILE_W / 2 + inset + fx * side, y0 + fy * side))
    return out


def svg():
    s = 1000
    c = s / 2
    r = lambda v: round(v * s, 1)
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {s} {s}">',
        f'<rect width="{s}" height="{s}" fill="{GREEN}"/>',
        f'<g transform="translate({c} {c}) rotate({-ANGLE})">',
        f'<rect x="{r(-TILE_W / 2)}" y="{r(-TILE_H / 2)}" width="{r(TILE_W)}" '
        f'height="{r(TILE_H)}" rx="{r(TILE_R)}" fill="{TILE}"/>',
        f'<line x1="{r(-TILE_W / 2 + 0.04)}" y1="0" x2="{r(TILE_W / 2 - 0.04)}" y2="0" '
        f'stroke="{LINE}" stroke-width="{r(0.012)}" stroke-linecap="round"/>',
    ]
    for x, y in pips():
        parts.append(f'<circle cx="{r(x)}" cy="{r(y)}" r="{r(PIP_R)}" fill="{PIP}"/>')
    parts += ["</g>", "</svg>"]
    return "\n".join(parts) + "\n"


def tile_layer(size, scale=1.0):
    """Só a pedra, num fundo transparente de `size` x `size`."""
    big = size * 4  # desenha maior e reduz, para as bordas ficarem lisas
    k = big * scale
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c = big / 2
    box = (c - k * TILE_W / 2, c - k * TILE_H / 2, c + k * TILE_W / 2, c + k * TILE_H / 2)
    d.rounded_rectangle(box, radius=k * TILE_R, fill=TILE)
    lw = k * 0.012
    d.rounded_rectangle(
        (c - k * (TILE_W / 2 - 0.04), c - lw / 2, c + k * (TILE_W / 2 - 0.04), c + lw / 2),
        radius=lw / 2, fill=LINE)
    for x, y in pips():
        px, py, pr = c + k * x, c + k * y, k * PIP_R
        d.ellipse((px - pr, py - pr, px + pr, py + pr), fill=PIP)
    img = img.rotate(ANGLE, resample=Image.BICUBIC, center=(c, c))
    return img.resize((size, size), Image.LANCZOS)


def on_green(layer):
    bg = Image.new("RGBA", layer.size, GREEN)
    bg.alpha_composite(layer)
    return bg


def feature():
    w, h = 1024, 500
    img = Image.new("RGBA", (w, h), GREEN)
    img.alpha_composite(tile_layer(h).crop((0, 0, h, h)), (w // 2 - h // 2, 0))
    # As bolinhas do placar, dos dois lados da pedra.
    big = 4
    dots = Image.new("RGBA", (w * big, h * big), (0, 0, 0, 0))
    d = ImageDraw.Draw(dots)
    for i in range(4):
        for side, color in ((-1, GOLD), (1, TILE)):
            x = (w / 2 + side * (210 + i * 62)) * big
            y = h / 2 * big
            rr = 20 * big
            d.ellipse((x - rr, y - rr, x + rr, y + rr), fill=color)
    img.alpha_composite(dots.resize((w, h), Image.LANCZOS))
    return img.convert("RGB")


def main():
    os.makedirs(os.path.join(OUT, "android"), exist_ok=True)
    with open(os.path.join(OUT, "icon.svg"), "w") as f:
        f.write(svg())
    on_green(tile_layer(512)).convert("RGB").save(os.path.join(OUT, "play-icon-512.png"))
    feature().save(os.path.join(OUT, "play-feature-1024x500.png"))
    # Launcher: ícone comum (48dp) e frente do adaptativo (108dp). No
    # adaptativo, só o círculo central de 66dp aparece com certeza; a pedra
    # inteira cabe nele com a escala 0.6.
    for name, mult in (("mdpi", 1), ("hdpi", 1.5), ("xhdpi", 2), ("xxhdpi", 3), ("xxxhdpi", 4)):
        folder = os.path.join(OUT, "android", f"mipmap-{name}")
        os.makedirs(folder, exist_ok=True)
        on_green(tile_layer(round(48 * mult))).save(os.path.join(folder, "ic_launcher.webp"))
        tile_layer(round(108 * mult), 0.6).save(
            os.path.join(folder, "ic_launcher_foreground.webp"))


if __name__ == "__main__":
    main()
