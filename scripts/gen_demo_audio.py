#!/usr/bin/env python3
"""合成 assets/demo/ 下的原创占位音效与 BGM（Python 标准库合成 WAV，oggenc 编码为 OGG Vorbis）。

用法：python3 scripts/gen_demo_audio.py        （需要 vorbis-tools 的 oggenc）
产物：
  assets/demo/blip.ogg   0.09 s 上扫正弦“哔”声（发射）
  assets/demo/hit.ogg    0.15 s 噪声 + 低频冲击（命中）
  assets/demo/loop.ogg   10 s 可无缝循环的方波 / 三角波小曲（120 BPM，5 小节）
"""

import math
import os
import struct
import subprocess
import tempfile
import wave

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "demo")
RATE = 22050


def write_ogg(name, samples):
    with tempfile.TemporaryDirectory() as tmp:
        wav_path = os.path.join(tmp, name + ".wav")
        with wave.open(wav_path, "wb") as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(RATE)
            w.writeframes(b"".join(struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767)) for s in samples))
        out = os.path.join(ROOT, name + ".ogg")
        subprocess.run(["oggenc", "-Q", "-q", "3", "-o", out, wav_path], check=True)


def blip():
    n = int(RATE * 0.09)
    out, phase = [], 0.0
    for i in range(n):
        t = i / n
        freq = 880.0 * (2.0 ** t)  # 880 → 1760 Hz
        phase += 2 * math.pi * freq / RATE
        env = min(1.0, i / 60) * math.exp(-4.0 * t)
        out.append(0.45 * env * math.sin(phase))
    return out


def hit():
    n = int(RATE * 0.15)
    out, seed = [], 12345
    for i in range(n):
        t = i / n
        seed = (seed * 1103515245 + 12345) & 0x7FFFFFFF  # 固定种子 LCG，结果可复现
        noise = seed / 0x3FFFFFFF - 1.0
        thump = math.sin(2 * math.pi * 110.0 * i / RATE)
        env = math.exp(-9.0 * t)
        out.append(env * (0.35 * noise + 0.4 * thump))
    return out


def note_freq(midi):
    return 440.0 * 2.0 ** ((midi - 69) / 12)


def loop():
    seconds = 10.0
    n = int(RATE * seconds)
    beat = 0.5  # 120 BPM
    out = [0.0] * n
    # 每小节 4 拍：低音（方波，每拍一音）+ 琶音（三角波，八分音符）
    bars = [(45, [57, 60, 64, 69]), (41, [53, 57, 60, 65]), (48, [60, 64, 67, 72]),
            (43, [55, 59, 62, 67]), (45, [57, 60, 64, 69])]

    def add_tone(start, dur, freq, amp, shape):
        s0, s1 = int(start * RATE), min(n, int((start + dur) * RATE))
        for i in range(s0, s1):
            t = (i - s0) / RATE
            env = min(1.0, t / 0.005) * min(1.0, (s1 - i) / (0.01 * RATE))
            ph = (freq * t) % 1.0
            if shape == "square":
                v = 1.0 if ph < 0.5 else -1.0
            else:
                v = 4.0 * abs(ph - 0.5) - 1.0
            out[i] += amp * env * v

    for bar, (bass, arp) in enumerate(bars):
        t0 = bar * 4 * beat
        for b in range(4):
            add_tone(t0 + b * beat, beat * 0.9, note_freq(bass), 0.12, "square")
        for k in range(8):
            add_tone(t0 + k * beat / 2, beat / 2 * 0.8, note_freq(arp[k % 4]), 0.18, "triangle")
    return out


def main():
    os.makedirs(ROOT, exist_ok=True)
    write_ogg("blip", blip())
    write_ogg("hit", hit())
    write_ogg("loop", loop())


if __name__ == "__main__":
    main()
