"""Render the SVG icon candidates to 1024px PNGs with headless Edge, then build a comparison sheet.

Usage: python design/icons/render.py            # renders all A-E + candidates.png
       python design/icons/render.py A-stack    # renders one
"""
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
CANDIDATES = [
    ("A-stack", "A 项目卡叠"),
    ("B-ring", "B 存储环"),
    ("C-pulse", "C 活跃脉搏"),
    ("D-grid", "D 清理网格"),
    ("E-prompt", "E 终端记忆"),
]


def render_svg(name: str, size: int = 1024) -> str:
    svg = os.path.join(HERE, f"{name}.svg")
    out = os.path.join(HERE, f"{name}.png")
    html = os.path.join(tempfile.gettempdir(), f"icon-{name}.html")
    with open(html, "w", encoding="utf-8") as f:
        f.write(
            "<!doctype html><html><head><meta charset='utf-8'><style>"
            "html,body{margin:0;padding:0;background:transparent;overflow:hidden}"
            f"img{{display:block;width:{size}px;height:{size}px}}"
            "</style></head><body>"
            f"<img src='file:///{svg.replace(os.sep, '/')}'></body></html>"
        )
    profile = os.path.join(tempfile.gettempdir(), "edge-icon-render-profile")
    cmd = [
        EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
        "--default-background-color=00000000",
        f"--user-data-dir={profile}", f"--window-size={size},{size}",
        f"--screenshot={out}", f"file:///{html.replace(os.sep, '/')}",
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120)
    img = Image.open(out).convert("RGBA")
    if img.size != (size, size):
        img = img.crop((0, 0, size, size))
        img.save(out)
    return out


def contact_sheet():
    font_path = r"C:\Windows\Fonts\msyh.ttc"
    title_font = ImageFont.truetype(font_path, 30)
    small_font = ImageFont.truetype(font_path, 20)
    cell_w, big, mid, small = 300, 200, 96, 48
    pad = 36
    width = pad + cell_w * len(CANDIDATES) + pad
    row_light_h, row_dark_h, row_small_h = 300, 300, 130
    height = 90 + row_light_h + row_dark_h + row_small_h + pad
    sheet = Image.new("RGBA", (width, height), (245, 245, 247, 255))
    d = ImageDraw.Draw(sheet)
    d.text((pad, 28), "CC Project Manager 图标候选 · 浅色 / 深色桌面 · 任务栏 96 px 与托盘 48 px 尺寸", font=title_font, fill=(30, 30, 40, 255))

    # dark band
    y_dark = 90 + row_light_h
    d.rectangle([0, y_dark, width, y_dark + row_dark_h], fill=(24, 26, 34, 255))
    # small band
    y_small = y_dark + row_dark_h
    d.rectangle([0, y_small, width, y_small + row_small_h], fill=(232, 233, 238, 255))

    for i, (name, label) in enumerate(CANDIDATES):
        icon = Image.open(os.path.join(HERE, f"{name}.png")).convert("RGBA")
        x0 = pad + i * cell_w
        cx = x0 + cell_w // 2
        # light row
        ic = icon.resize((big, big), Image.LANCZOS)
        sheet.alpha_composite(ic, (cx - big // 2, 90 + 20))
        d.text((cx, 90 + 20 + big + 18), label, font=title_font, fill=(30, 30, 40, 255), anchor="mt")
        # dark row
        sheet.alpha_composite(ic, (cx - big // 2, y_dark + 40))
        d.text((cx, y_dark + 40 + big + 18), label, font=small_font, fill=(220, 222, 230, 255), anchor="mt")
        # small row: 96 and 48 side by side
        im = icon.resize((mid, mid), Image.LANCZOS)
        ismall = icon.resize((small, small), Image.LANCZOS)
        sheet.alpha_composite(im, (cx - (mid + 20 + small) // 2, y_small + 16))
        sheet.alpha_composite(ismall, (cx - (mid + 20 + small) // 2 + mid + 20, y_small + 16 + (mid - small) // 2))
        d.text((cx, y_small + 16 + mid + 4), "96 px · 48 px", font=small_font, fill=(90, 90, 100, 255), anchor="mt")

    out = os.path.join(HERE, "candidates.png")
    sheet.convert("RGB").save(out, optimize=True)
    return out


if __name__ == "__main__":
    only = sys.argv[1] if len(sys.argv) > 1 else None
    for name, _ in CANDIDATES:
        if only and name != only:
            continue
        print("rendered", render_svg(name))
    if not only:
        print("sheet", contact_sheet())
