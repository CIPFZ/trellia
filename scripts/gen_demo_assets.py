#!/usr/bin/env python3
"""生成 assets/demo/ 下的原创几何占位 PNG（仅用 Python 标准库）。

用法：python3 scripts/gen_demo_assets.py
产物：
  assets/demo/placeholder_8x8.png  8 x 8 品红 / 黑棋盘格
  assets/demo/walker.png           128 x 192，2 x 2 排布的 4 帧 64 x 96 几何人形
                                   （walk_0 / walk_1 / bite_0 / bite_1，见 ADR-004）
"""

import os
import struct
import zlib

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "demo")


def write_png(path, width, height, pixels):
    """pixels: 长度 width*height 的 (r, g, b, a) 列表，行优先。"""
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # 过滤类型 None
        for x in range(width):
            raw.extend(pixels[y * width + x])

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


class Canvas:
    def __init__(self, width, height):
        self.width = width
        self.height = height
        self.pixels = [(0, 0, 0, 0)] * (width * height)

    def rect(self, x0, y0, x1, y1, color):
        for y in range(max(0, y0), min(self.height, y1)):
            for x in range(max(0, x0), min(self.width, x1)):
                self.pixels[y * self.width + x] = color

    def circle(self, cx, cy, r, color):
        for y in range(cy - r, cy + r + 1):
            for x in range(cx - r, cx + r + 1):
                if 0 <= x < self.width and 0 <= y < self.height and (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                    self.pixels[y * self.width + x] = color


BODY = (142, 68, 173, 255)
HEAD = (187, 143, 206, 255)
LIMB = (108, 52, 131, 255)
DARK = (30, 20, 40, 255)


def walker_frame(c, ox, oy, pose):
    """在 (ox, oy) 处画一帧 64 x 96 的几何人形。行者朝左（-x）前进。"""
    # 腿：walk 两帧交替，bite 两帧站定
    if pose == "walk_0":
        c.rect(ox + 20, oy + 66, ox + 29, oy + 94, LIMB)
        c.rect(ox + 35, oy + 66, ox + 44, oy + 88, LIMB)
    elif pose == "walk_1":
        c.rect(ox + 20, oy + 66, ox + 29, oy + 88, LIMB)
        c.rect(ox + 35, oy + 66, ox + 44, oy + 94, LIMB)
    else:
        c.rect(ox + 20, oy + 66, ox + 29, oy + 94, LIMB)
        c.rect(ox + 35, oy + 66, ox + 44, oy + 94, LIMB)
    # 躯干与头
    c.rect(ox + 18, oy + 32, ox + 46, oy + 68, BODY)
    c.circle(ox + 32, oy + 18, 13, HEAD)
    c.rect(ox + 22, oy + 14, ox + 27, oy + 19, DARK)  # 眼
    # 手臂与嘴
    if pose.startswith("walk"):
        c.rect(ox + 8, oy + 38, ox + 18, oy + 46, LIMB)
        c.rect(ox + 21, oy + 24, ox + 29, oy + 27, DARK)
    elif pose == "bite_0":
        c.rect(ox + 2, oy + 36, ox + 18, oy + 44, LIMB)
        c.rect(ox + 19, oy + 23, ox + 29, oy + 27, DARK)
    else:  # bite_1：张嘴前倾
        c.rect(ox + 0, oy + 40, ox + 18, oy + 48, LIMB)
        c.rect(ox + 19, oy + 22, ox + 30, oy + 31, DARK)


def main():
    os.makedirs(ROOT, exist_ok=True)

    checker = Canvas(8, 8)
    for y in range(8):
        for x in range(8):
            on = (x // 4 + y // 4) % 2 == 0
            checker.pixels[y * 8 + x] = (255, 0, 255, 255) if on else (0, 0, 0, 255)
    write_png(os.path.join(ROOT, "placeholder_8x8.png"), 8, 8, checker.pixels)

    walker = Canvas(128, 192)
    walker_frame(walker, 0, 0, "walk_0")
    walker_frame(walker, 64, 0, "walk_1")
    walker_frame(walker, 0, 96, "bite_0")
    walker_frame(walker, 64, 96, "bite_1")
    write_png(os.path.join(ROOT, "walker.png"), 128, 192, walker.pixels)


if __name__ == "__main__":
    main()
