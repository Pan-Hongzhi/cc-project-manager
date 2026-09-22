import os, struct, zlib
W = H = 512
def chunk(t, d):
    c = struct.pack('>I', len(d)) + t + d
    return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
raw = bytearray()
for y in range(H):
    raw += b'\x00'
    for x in range(W):
        inside = 48 <= x < 464 and 48 <= y < 464
        raw += bytes((0x2f, 0x6f, 0xed, 0xff)) if inside else bytes((0, 0, 0, 0))
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 6, 0, 0, 0)) \
    + chunk(b'IDAT', zlib.compress(bytes(raw), 9)) + chunk(b'IEND', b'')
os.makedirs('src-tauri/icons', exist_ok=True)
open('src-tauri/icons/source.png', 'wb').write(png)
print('ok')
