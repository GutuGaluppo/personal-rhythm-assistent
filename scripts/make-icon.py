"""Generate a placeholder 1024x1024 app icon (soft circle). Pure stdlib."""
import struct, sys, zlib

N = 1024
cx = cy = N / 2
bg = (246, 244, 239, 255)
fg = (79, 127, 138, 255)
rows = []
for y in range(N):
    row = bytearray([0])
    for x in range(N):
        d = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
        row += bytes(fg if d < N * 0.36 else bg)
    rows.append(bytes(row))

def chunk(t, d):
    c = struct.pack(">I", len(d)) + t + d
    return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")
open(sys.argv[1], "wb").write(png)
