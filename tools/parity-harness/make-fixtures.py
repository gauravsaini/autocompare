"""Generate clean-room parity fixtures (original byte patterns only)."""
import struct, wave, zlib
from pathlib import Path

F = Path(__file__).parent / "fixtures"
shutil_rm = __import__("shutil").rmtree
if F.exists():
    shutil_rm(F)

(F / "folder" / "left").mkdir(parents=True)
(F / "folder" / "right").mkdir(parents=True)
(F / "sync" / "left").mkdir(parents=True)
(F / "sync" / "right").mkdir(parents=True)
for d in ["fmerge-base", "fmerge-left", "fmerge-right"]:
    (F / d).mkdir(parents=True)

(F / "t_left.txt").write_text("alpha\nbeta\ngamma\n")
(F / "t_right.txt").write_text("alpha\nBETA\ngamma\n")
(F / "t_same.txt").write_text("alpha\nbeta\ngamma\n")
(F / "h_left.bin").write_bytes(b"hello world")
(F / "h_right.bin").write_bytes(b"hallo world")
(F / "m_base.txt").write_text("base\n")
(F / "m_left.txt").write_text("base\nleft\n")
(F / "m_right.txt").write_text("base\nright\n")
(F / "tab_left.csv").write_text("id,name,role\n1,amy,dev\n2,bob,ops\n")
(F / "tab_right.csv").write_text("id,name,role\n1,amy,dev\n2,bob,lead\n")
(F / "folder" / "left" / "f.txt").write_text("same\n")
(F / "folder" / "right" / "f.txt").write_text("same\n")
(F / "folder" / "right" / "extra.txt").write_text("only right\n")
(F / "sync" / "left" / "common.txt").write_text("x\n")
(F / "sync" / "left" / "extra.txt").write_text("new\n")
(F / "sync" / "right" / "common.txt").write_text("x\n")
for d, c in [("fmerge-base", "v1\n"), ("fmerge-left", "v1\nL\n"), ("fmerge-right", "v1\nR\n")]:
    (F / d / "doc.txt").write_text(c)

def png(path, rgb):
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    raw = b"".join(b"\x00" + b"".join(bytes(px) for px in row) for row in rgb)
    ihdr = struct.pack(">IIBBBBB", 4, 4, 8, 2, 0, 0, 0)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
                     + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))

red, blue = (255, 0, 0), (0, 0, 255)
png(F / "pic_left.png", [[red] * 4 for _ in range(4)])
grid = [[red] * 4 for _ in range(4)]
grid[1][1] = blue
png(F / "pic_right.png", grid)

def wav(path, freq):
    import math
    w = wave.open(str(path), "w")
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(8000)
    w.writeframes(b"".join(struct.pack("<h", int(9000 * math.sin(2 * math.pi * freq * t / 8000)))
                            for t in range(2000)))
    w.close()

wav(F / "a_left.wav", 440)
wav(F / "a_right.wav", 880)
print(f"fixtures written to {F}")
