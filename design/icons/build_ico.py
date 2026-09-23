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
# 覆盖 100% / 125% / 150% / 175% / 200% 缩放下的标题栏（16/20/24/28/32）与任务栏（32/40/48/56/64）尺寸
SMALL_SIZES = [16, 20, 24, 28, 32, 40, 48, 56]
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


def _dib_frame(img: Image.Image) -> bytes:
    """32 位 BGRA DIB + 1 位 AND 掩码，行序自下而上，biHeight 为两倍高度（ICO 约定）。"""
    import struct

    w, h = img.size
    rgba = img.convert("RGBA")
    xor_rows = []
    and_rows = []
    mask_stride = ((w + 31) // 32) * 4
    for y in range(h - 1, -1, -1):  # bottom-up
        row = bytearray()
        mask = bytearray(mask_stride)
        for x in range(w):
            r, g, b, a = rgba.getpixel((x, y))
            row += bytes((b, g, r, a))
            if a == 0:
                mask[x // 8] |= 0x80 >> (x % 8)  # 1 = 透明
        xor_rows.append(bytes(row))
        and_rows.append(bytes(mask))
    xor = b"".join(xor_rows)
    and_mask = b"".join(and_rows)
    header = struct.pack("<IiiHHIIiiII", 40, w, h * 2, 1, 32, 0, len(xor) + len(and_mask), 0, 0, 0, 0)
    return header + xor + and_mask


def write_ico(frames: dict, path: str) -> None:
    import io
    import struct

    sizes = sorted(frames)
    blobs = []
    for s in sizes:
        img = frames[s]
        if s >= 256:
            buf = io.BytesIO()
            img.convert("RGBA").save(buf, format="PNG")
            blobs.append(buf.getvalue())
        else:
            blobs.append(_dib_frame(img))
    header = struct.pack("<HHH", 0, 1, len(sizes))
    offset = 6 + 16 * len(sizes)
    entries = b""
    for s, blob in zip(sizes, blobs):
        dim = 0 if s >= 256 else s
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset)
        offset += len(blob)
    with open(path, "wb") as f:
        f.write(header + entries + b"".join(blobs))


def main():
    import sys
    name = sys.argv[1] if len(sys.argv) > 1 else "A-stack"   # e.g. A-stack, D-grid
    # 第二个参数可指定小尺寸帧用哪个 SVG；默认用 <name>-small.svg（简化版），
    # 传 <name>.svg 则各尺寸都用精细版原生渲染
    small_svg = sys.argv[2] if len(sys.argv) > 2 else f"{name}-small.svg"
    large = Image.open(os.path.join(HERE, f"{name}.png")).convert("RGBA")
    frames = {s: render_at(small_svg, s) for s in SMALL_SIZES}
    for s in LARGE_SIZES:
        frames[s] = large.resize((s, s), Image.LANCZOS)

    # icon.ico with distinct artwork per size (Pillow embeds each appended image at its own size)
    # Pillow 写出的 ico 帧 Windows LoadImage 读不对（画成纯色方块），这里按经典 ICO 格式手写：
    # 小于 256 px 的帧用 32 位 DIB（含 AND 掩码），256 px 帧用 PNG。
    ico_path = os.path.join(ICONS, "icon.ico")
    write_ico(frames, ico_path)
    # 供运行时直接创建窗口图标的单尺寸 PNG（src-tauri/src/crisp_icon.rs 用 include_bytes! 内嵌）
    win_dir = os.path.join(ICONS, "win")
    os.makedirs(win_dir, exist_ok=True)
    for s in SMALL_SIZES + [64]:
        frames[s].save(os.path.join(win_dir, f"{s}.png"))
    frames[32].save(os.path.join(ICONS, "32x32.png"))
    print("wrote", ico_path, "sizes", sorted(frames))

    # before/after sheet: old = detailed art downscaled, new = small-size art, both zoomed 6x
    font = ImageFont.truetype(r"C:\Windows\Fonts\msyh.ttc", 22)
    zoom = 6
    cols = [16, 24, 32, 48]
    cell = 48 * zoom + 40
    sheet = Image.new("RGBA", (120 + cell * len(cols), 60 + 2 * (cell + 40)), (245, 245, 247, 255))
    d = ImageDraw.Draw(sheet)
    d.text((20, 18), f"{name} 图标小尺寸对比（放大 6 倍）· 上：原精细版缩放 · 下：小尺寸专用版", font=font, fill=(30, 30, 40, 255))
    for r, (label, src) in enumerate([("原版", None), ("新版", "small")]):
        y = 60 + r * (cell + 40)
        d.text((20, y + cell // 2 - 12), label, font=font, fill=(30, 30, 40, 255))
        for c, s in enumerate(cols):
            im = large.resize((s, s), Image.LANCZOS) if src is None else frames[s]
            big = im.resize((s * zoom, s * zoom), Image.NEAREST)
            x = 120 + c * cell + (cell - 40 - s * zoom) // 2
            sheet.alpha_composite(big, (x, y + (cell - 40 - s * zoom) // 2))
            d.text((120 + c * cell + (cell - 40) // 2, y + cell - 30), f"{s} px", font=font, fill=(90, 90, 100, 255), anchor="mt")
    out = os.path.join(HERE, f"small-sizes-{name}.png")
    sheet.convert("RGB").save(out)
    print("sheet", out)


if __name__ == "__main__":
    main()
