"""Build src-tauri/icons/icon.ico with size-specific artwork.

16/20/24/32/48 px come from the simplified small-size SVG (A-stack-small.svg), each rendered
natively at its target size by headless Edge; 64/128/256 px come from the detailed A-stack.png.
Also refreshes 32x32.png and writes a zoomed before/after sheet (small-sizes.png).

Usage: python design/icons/build_ico.py
"""
import os
import shutil
import subprocess
import tempfile

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
ICONS = os.path.join(ROOT, "src-tauri", "icons")
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
SMALL_SIZES = [16, 20, 24, 32, 48]
LARGE_SIZES = [64, 128, 256]


def render_at(svg_name: str, size: int) -> Image.Image:
    svg = os.path.join(HERE, svg_name)
    out = os.path.join(tempfile.gettempdir(), f"{svg_name}-{size}.png")
    html = os.path.join(tempfile.gettempdir(), f"{svg_name}-{size}.html")
    with open(html, "w", encoding="utf-8") as f:
        f.write(
            "<!doctype html><html><head><meta charset='utf-8'><style>"
            "html,body{margin:0;padding:0;background:transparent;overflow:hidden}"
            f"img{{display:block;width:{size}px;height:{size}px}}</style></head><body>"
            f"<img src='file:///{svg.replace(os.sep, '/')}'></body></html>"
        )
    profile = os.path.join(tempfile.gettempdir(), "edge-icon-render-profile")
    subprocess.run(
        [EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
         "--default-background-color=00000000", f"--user-data-dir={profile}",
         f"--window-size={size},{size}", f"--screenshot={out}", f"file:///{html.replace(os.sep, '/')}"],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120)
    return Image.open(out).convert("RGBA").crop((0, 0, size, size))


def main():
    large = Image.open(os.path.join(HERE, "A-stack.png")).convert("RGBA")
    frames = {s: render_at("A-stack-small.svg", s) for s in SMALL_SIZES}
    for s in LARGE_SIZES:
        frames[s] = large.resize((s, s), Image.LANCZOS)

    # icon.ico with distinct artwork per size (Pillow embeds each appended image at its own size)
    # Pillow drops any size larger than the base image, so the 256 px frame must be the base.
    sizes = sorted(frames)
    base = frames[sizes[-1]]
    others = [frames[s] for s in sizes[:-1]]
    ico_path = os.path.join(ICONS, "icon.ico")
    base.save(ico_path, format="ICO", sizes=[(s, s) for s in sizes], append_images=others)
    frames[32].save(os.path.join(ICONS, "32x32.png"))
    print("wrote", ico_path, "sizes", sorted(frames))

    # before/after sheet: old = detailed art downscaled, new = small-size art, both zoomed 6x
    font = ImageFont.truetype(r"C:\Windows\Fonts\msyh.ttc", 22)
    zoom = 6
    cols = [16, 24, 32, 48]
    cell = 48 * zoom + 40
    sheet = Image.new("RGBA", (120 + cell * len(cols), 60 + 2 * (cell + 40)), (245, 245, 247, 255))
    d = ImageDraw.Draw(sheet)
    d.text((20, 18), "A 图标小尺寸对比（放大 6 倍）· 上：原精细版缩放 · 下：小尺寸专用版", font=font, fill=(30, 30, 40, 255))
    for r, (label, src) in enumerate([("原版", None), ("新版", "small")]):
        y = 60 + r * (cell + 40)
        d.text((20, y + cell // 2 - 12), label, font=font, fill=(30, 30, 40, 255))
        for c, s in enumerate(cols):
            im = large.resize((s, s), Image.LANCZOS) if src is None else frames[s]
            big = im.resize((s * zoom, s * zoom), Image.NEAREST)
            x = 120 + c * cell + (cell - 40 - s * zoom) // 2
            sheet.alpha_composite(big, (x, y + (cell - 40 - s * zoom) // 2))
            d.text((120 + c * cell + (cell - 40) // 2, y + cell - 30), f"{s} px", font=font, fill=(90, 90, 100, 255), anchor="mt")
    out = os.path.join(HERE, "small-sizes.png")
    sheet.convert("RGB").save(out)
    print("sheet", out)


if __name__ == "__main__":
    main()
