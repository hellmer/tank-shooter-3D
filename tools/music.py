"""Synthesizes the game's music: two loops of the same length that play together.

Usage:  python3 tools/music.py assets/music

- calm.ogg: pads, a pulsing bass and an arpeggio. Always playing, quietly.
- combat.ogg: drums, a driving distorted bass and power-chord stabs. The game fades it in
  when enemies are close, so the music builds up during a fight without ever cutting.

Both are in D minor at 96 BPM, 16 bars (40 s), and loop seamlessly. Everything is built from
oscillators, noise and envelopes with fixed seeds, so running this again gives the same music.
Needs Python 3 with numpy, and ffmpeg (with libvorbis) to write .ogg; without ffmpeg it writes .wav.
"""
import os
import shutil
import subprocess
import sys
import wave

import numpy as np

RATE = 32000
BPM = 96
BEAT = 60 / BPM
BARS = 16
LENGTH = int(round(BARS * 4 * BEAT * RATE))

# Two bars per chord: Dm, Bb, Gm, A (the A major gives the loop its pull back to Dm).
CHORDS = [
    ("D", [50, 53, 57]),
    ("Bb", [46, 50, 53]),
    ("G", [43, 46, 50]),
    ("A", [45, 49, 52]),
]


def hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def chord_at(bar):
    return CHORDS[(bar // 2) % len(CHORDS)]


def at(beat):
    return int(round(beat * BEAT * RATE))


def env_adsr(n, a, d, s, r, hold):
    """Envelope for `n` samples: attack, decay to sustain, hold, release (all in seconds)."""
    t = np.arange(n) / RATE
    e = np.where(t < a, t / max(a, 1e-6), s + (1 - s) * np.exp(-(t - a) / max(d, 1e-6)))
    end = a + hold
    e = np.where(t > end, e * np.exp(-(t - end) / max(r, 1e-6)), e)
    return e


def saw(f, n, phase=0.0, top=6000):
    """Band-limited sawtooth: harmonics up to `top` Hz."""
    t = np.arange(n) / RATE
    out = np.zeros(n)
    k = 1
    while k * f < top and k < 60:
        out += np.sin(2 * np.pi * k * f * t + phase * k) / k
        k += 1
    return out * 0.6


def square(f, n, top=6000):
    t = np.arange(n) / RATE
    out = np.zeros(n)
    k = 1
    while k * f < top and k < 60:
        out += np.sin(2 * np.pi * k * f * t) / k
        k += 2
    return out * 0.8


def tri(f, n):
    t = np.arange(n) / RATE
    return 2 * np.abs(2 * ((t * f) % 1) - 1) - 1


def lowpass(x, cutoff):
    cutoff = np.broadcast_to(np.asarray(cutoff, float), x.shape)
    a = 1 - np.exp(-2 * np.pi * cutoff / RATE)
    y = np.empty_like(x)
    acc = 0.0
    for i in range(len(x)):
        acc += a[i] * (x[i] - acc)
        y[i] = acc
    return y


def highpass(x, cutoff):
    return x - lowpass(x, cutoff)


def place(track, start, sound, gain=1.0, pan=0.0):
    """Adds a mono sound into a stereo track, wrapping past the end so the loop is seamless."""
    left = gain * np.cos((pan + 1) * np.pi / 4)
    right = gain * np.sin((pan + 1) * np.pi / 4)
    idx = (start + np.arange(len(sound))) % LENGTH
    np.add.at(track[:, 0], idx, sound * left)
    np.add.at(track[:, 1], idx, sound * right)


def echo(track, beats, feedback, mix):
    """Ping-pong delay, wrapped around the loop."""
    d = at(beats)
    out = track.copy()
    tap = track.copy()
    for k in range(1, 6):
        tap = np.roll(tap[:, ::-1], d, axis=0) * feedback
        out += tap * mix
    return out


def soft_clip(x, drive):
    return np.tanh(x * drive) / np.tanh(drive)


def normalize(x, peak):
    m = np.abs(x).max()
    return x if m == 0 else x * (peak / m)


# ---------------------------------------------------------------- calm layer

def calm():
    track = np.zeros((LENGTH, 2))
    rng = np.random.default_rng(3)
    # Pads: each chord held for two bars, two detuned saws per note, slow swell, filter opening.
    for bar in range(0, BARS, 2):
        _, notes = chord_at(bar)
        n = at(8) + at(2)
        for note in notes:
            for side, detune in ((-0.6, 0.997), (0.6, 1.003)):
                v = saw(hz(note) * detune, n, phase=rng.uniform(0, 6.28), top=3000)
                cut = 500 + 700 * (0.5 - 0.5 * np.cos(np.linspace(0, np.pi, n)))
                v = lowpass(v, cut) * env_adsr(n, 1.2, 2.0, 0.8, 0.9, 7.5 * BEAT)
                place(track, at(bar * 4), v, 0.11, side)
    # Bass: root in octave 2, steady eighths, a little louder on the beat.
    for bar in range(BARS):
        _, notes = chord_at(bar)
        root = notes[0] - 12 if notes[0] >= 48 else notes[0]
        for e in range(8):
            n = at(0.5)
            v = (np.sin(2 * np.pi * hz(root) * np.arange(n) / RATE) + 0.3 * square(hz(root), n, top=900))
            v *= env_adsr(n, 0.005, 0.12, 0.35, 0.05, 0.18)
            place(track, at(bar * 4 + e * 0.5), v, 0.32 if e % 2 == 0 else 0.22)
    # Arpeggio in the second half: chord tones up and down in sixteenths, through an echo.
    arp = np.zeros((LENGTH, 2))
    pattern = [0, 1, 2, 1, 0, 2, 1, 2]
    for bar in range(8, BARS):
        _, notes = chord_at(bar)
        for s in range(16):
            note = notes[pattern[s % 8]] + 12
            n = at(0.25)
            v = tri(hz(note), n) * env_adsr(n, 0.003, 0.08, 0.0, 0.05, 0.0)
            place(arp, at(bar * 4 + s * 0.25), v, 0.09, 0.3 if s % 2 else -0.3)
    track += echo(arp, 0.75, 0.45, 0.6)
    return normalize(track, 0.8)


# ---------------------------------------------------------------- combat layer

def kick():
    n = int(0.35 * RATE)
    t = np.arange(n) / RATE
    f = 45 + 110 * np.exp(-t / 0.03)
    body = np.sin(2 * np.pi * np.cumsum(f) / RATE) * np.exp(-t / 0.16)
    click = np.random.default_rng(1).uniform(-1, 1, n) * np.exp(-t / 0.003) * 0.4
    return soft_clip(body + click, 1.5)


def snare():
    n = int(0.3 * RATE)
    t = np.arange(n) / RATE
    noise = highpass(np.random.default_rng(2).uniform(-1, 1, n), 1500) * np.exp(-t / 0.09)
    tone = np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.05)
    return noise * 0.9 + tone * 0.6


def hat(open_=False):
    n = int((0.25 if open_ else 0.06) * RATE)
    t = np.arange(n) / RATE
    noise = highpass(np.random.default_rng(4 + open_).uniform(-1, 1, n), 7000)
    return noise * np.exp(-t / (0.08 if open_ else 0.018))


def tom(f):
    n = int(0.4 * RATE)
    t = np.arange(n) / RATE
    ff = f * (1 + 0.5 * np.exp(-t / 0.04))
    return np.sin(2 * np.pi * np.cumsum(ff) / RATE) * np.exp(-t / 0.18)


def combat():
    track = np.zeros((LENGTH, 2))
    K, S, H, HO = kick(), snare(), hat(), hat(True)
    for bar in range(BARS):
        b = bar * 4
        last_of_four = bar % 4 == 3
        # Kick: 1, the "and" of 2, 3 (a march with a push).
        for beat in (0, 1.5, 2):
            place(track, at(b + beat), K, 0.9)
        if not last_of_four:
            place(track, at(b + 3.5), K, 0.6)
        for beat in (1, 3):
            place(track, at(b + beat), S, 0.55, 0.1)
        for e in range(8):
            open_ = e == 7 and not last_of_four
            place(track, at(b + e * 0.5), HO if open_ else H, 0.16 if e % 2 else 0.24, -0.35)
        if last_of_four:
            # Tom fill into the next phrase.
            for k, (beat, f) in enumerate(((3.0, 160), (3.25, 140), (3.5, 115), (3.75, 95))):
                place(track, at(b + beat), tom(f), 0.5, 0.4 - k * 0.25)
    # Driving bass: sixteenths on the root, octave jumps on the off-beats, distorted.
    bass = np.zeros((LENGTH, 2))
    for bar in range(BARS):
        _, notes = chord_at(bar)
        root = notes[0] - 12 if notes[0] >= 48 else notes[0]
        for s in range(16):
            note = root + (12 if s % 4 == 3 else 0)
            n = at(0.25)
            v = saw(hz(note), n, top=2500) * env_adsr(n, 0.002, 0.07, 0.4, 0.02, 0.1)
            place(bass, at(bar * 4 + s * 0.25), v, 0.5 if s % 4 == 0 else 0.35)
    bass[:, 0] = lowpass(soft_clip(bass[:, 0], 2.5), 1400)
    bass[:, 1] = lowpass(soft_clip(bass[:, 1], 2.5), 1400)
    track += bass * 0.8
    # Power-chord stabs on each chord change, brassy and wide.
    for bar in range(0, BARS, 2):
        _, notes = chord_at(bar)
        for beat, length in ((0, 1.5), (1.5, 0.5), (6, 1.0)):
            n = at(length)
            for note, pan in ((notes[0], -0.5), (notes[0] + 7, 0.5), (notes[0] + 12, 0.0)):
                v = saw(hz(note), n, top=4000)
                v = lowpass(v, 900 + 2500 * np.exp(-np.arange(n) / RATE / 0.15))
                v *= env_adsr(n, 0.01, 0.2, 0.5, 0.08, length * BEAT - 0.08)
                place(track, at(bar * 4 + beat), v, 0.16, pan)
    return normalize(soft_clip(track, 1.2), 0.8)


# ---------------------------------------------------------------- output

def write(path_base, x):
    data = (np.clip(x, -1, 1) * 32767).astype("<i2")
    wav = path_base + ".wav"
    with wave.open(wav, "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(data.tobytes())
    if shutil.which("ffmpeg"):
        ogg = path_base + ".ogg"
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", wav, "-c:a", "libvorbis", "-q:a", "4", ogg], check=True)
        os.remove(wav)
        return ogg
    return wav


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "assets/music"
    os.makedirs(out, exist_ok=True)
    for name, make in (("calm", calm), ("combat", combat)):
        path = write(os.path.join(out, name), make())
        print(f"{path}: {LENGTH / RATE:.1f} s")


if __name__ == "__main__":
    main()
