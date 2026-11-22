"""Synthesizes the game's sound effects into WAV files (no recordings needed).

Usage:  python3 tools/sfx.py assets/sounds

Every sound is built from sine waves, filtered noise and envelopes, with a fixed random seed,
so running it again gives the same files. Needs Python 3 with numpy.
"""
import os
import sys
import wave

import numpy as np

RATE = 22050


def t_axis(seconds):
    return np.arange(int(seconds * RATE)) / RATE


def noise(seconds, seed):
    return np.random.default_rng(seed).uniform(-1, 1, int(seconds * RATE))


def lowpass(x, cutoff):
    """One-pole low-pass. `cutoff` may be a number or an array (a sweep), in Hz."""
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


def decay(t, tau):
    return np.exp(-t / tau)


def attack(t, seconds):
    return np.clip(t / seconds, 0, 1)


def sweep_sine(t, f0, f1, tau):
    """Sine whose pitch falls from f0 to f1 with time constant tau."""
    f = f1 + (f0 - f1) * np.exp(-t / tau)
    return np.sin(2 * np.pi * np.cumsum(f) / RATE)


def pad(x, seconds):
    return np.concatenate([x, np.zeros(int(seconds * RATE))])


def mix(*parts):
    n = max(len(p) for p in parts)
    out = np.zeros(n)
    for p in parts:
        out[:len(p)] += p
    return out


def normalize(x, peak=0.9):
    m = np.abs(x).max()
    return x if m == 0 else x * (peak / m)


def soft_clip(x, drive=1.5):
    return np.tanh(x * drive) / np.tanh(drive)


# ---------------------------------------------------------------- sounds

def cannon(seed, low=48, body=0.9, crack=1.0, seconds=1.4):
    """Tank gun: a sharp crack, a deep falling thump and a rolling tail of filtered noise."""
    t = t_axis(seconds)
    n = noise(seconds, seed)
    crack_part = highpass(n, 900) * decay(t, 0.018) * crack
    thump = sweep_sine(t, low * 3, low, 0.05) * decay(t, 0.22) * body
    roll = lowpass(n, 2200 * decay(t, 0.25) + 120) * decay(t, 0.45) * 1.4
    return normalize(soft_clip(crack_part + thump + roll, 2.0))


def turret_shot():
    """The turrets' lighter guns: short, higher and snappier."""
    t = t_axis(0.6)
    n = noise(0.6, 11)
    crack = highpass(n, 1500) * decay(t, 0.012)
    thump = sweep_sine(t, 260, 90, 0.03) * decay(t, 0.09) * 0.8
    tail = lowpass(n, 2600 * decay(t, 0.12) + 200) * decay(t, 0.16)
    return normalize(soft_clip(crack + thump + tail, 1.8))


def explosion(seed, seconds, size):
    """Blast: a thump, a bright burst and a long rumble with crackles in it."""
    t = t_axis(seconds)
    n = noise(seconds, seed)
    thump = sweep_sine(t, 90, 30, 0.08 * size) * decay(t, 0.25 * size) * 1.2
    burst = lowpass(n, 4000 * decay(t, 0.08 * size) + 300) * decay(t, 0.12 * size)
    rumble = lowpass(n, 220) * decay(t, 0.55 * size) * attack(t, 0.03) * 3.0
    rng = np.random.default_rng(seed + 1)
    crackle = np.zeros_like(t)
    for _ in range(int(25 * size)):
        at = int(rng.uniform(0.05, seconds * 0.7) * RATE)
        k = np.arange(min(200, len(t) - at))
        crackle[at:at + len(k)] += rng.uniform(-1, 1, len(k)) * np.exp(-k / 40) * rng.uniform(0.2, 0.6)
    crackle *= decay(t, 0.4 * size)
    return normalize(soft_clip(thump + burst + rumble + highpass(crackle, 1000) * 0.5, 1.6))


def clang(seed, base, seconds, ring):
    """Shell on armor: inharmonic metal partials over a noise hit."""
    t = t_axis(seconds)
    rng = np.random.default_rng(seed)
    hit = highpass(noise(seconds, seed), 1200) * decay(t, 0.015) * 1.2
    tone = np.zeros_like(t)
    for ratio, amp in ((1.0, 1.0), (2.76, 0.6), (5.40, 0.35), (8.93, 0.2)):
        f = base * ratio * rng.uniform(0.98, 1.02)
        tone += np.sin(2 * np.pi * f * t + rng.uniform(0, 6.28)) * amp * decay(t, ring / ratio ** 0.5)
    thud = sweep_sine(t, 160, 60, 0.03) * decay(t, 0.08)
    return normalize(soft_clip(hit + tone * 0.6 + thud, 1.5))


def crate_break():
    """Wood: a hollow knock and a handful of splintering cracks."""
    t = t_axis(0.8)
    knock = sweep_sine(t, 220, 120, 0.04) * decay(t, 0.07)
    rng = np.random.default_rng(31)
    cracks = np.zeros_like(t)
    for _ in range(9):
        at = int(rng.uniform(0.0, 0.35) * RATE)
        k = np.arange(min(900, len(t) - at))
        cracks[at:at + len(k)] += rng.uniform(-1, 1, len(k)) * np.exp(-k / rng.uniform(60, 250))
    cracks = lowpass(highpass(cracks, 500), 3500)
    return normalize(knock * 0.8 + cracks)


def reload_click():
    """Breech closing: two short metallic clicks."""
    t = t_axis(0.28)
    out = np.zeros_like(t)
    for at, f, amp in ((0.0, 2100, 0.7), (0.11, 1500, 1.0)):
        k = t[int(at * RATE):] - at
        click = (np.sin(2 * np.pi * f * k) + highpass(noise(len(k) / RATE, int(f)), 2000) * 0.6) * decay(k, 0.012)
        out[int(at * RATE):] += click * amp
    return normalize(out, 0.7)


def engine_loop(seed, base, seconds=1.0):
    """Diesel rumble that loops cleanly: every component repeats a whole number of times."""
    n = int(seconds * RATE)
    t = np.arange(n) / RATE

    def whole(f):
        return round(f * seconds) / seconds

    fire = whole(base)  # cylinder firing rate
    pulses = 0.5 + 0.5 * np.cos(2 * np.pi * fire * t)
    pulses = pulses ** 3
    tone = np.zeros(n)
    for k, amp in ((1, 1.0), (2, 0.55), (3, 0.3), (5, 0.15)):
        tone += np.sin(2 * np.pi * whole(fire * k) * t) * amp
    # Noise made periodic by building it in the frequency domain.
    rng = np.random.default_rng(seed)
    spec = np.zeros(n // 2 + 1, complex)
    freqs = np.fft.rfftfreq(n, 1 / RATE)
    band = (freqs > 40) & (freqs < 700)
    spec[band] = rng.normal(size=band.sum()) + 1j * rng.normal(size=band.sum())
    rumble = np.fft.irfft(spec, n)
    rumble /= np.abs(rumble).max()
    # Track clatter: soft ticks at a whole-number rate.
    clatter_rate = whole(base * 0.75)
    phase = (t * clatter_rate) % 1.0
    clatter = np.exp(-phase / 0.03) * np.sin(2 * np.pi * 900 * t) * 0.15
    out = tone * (0.4 + 0.6 * pulses) * 0.6 + rumble * (0.5 + 0.5 * pulses) * 0.7 + clatter
    return normalize(soft_clip(out, 1.4), 0.8)


SOUNDS = {
    "cannon_player": lambda: pad(cannon(1, low=46, body=1.0, crack=1.0, seconds=1.5), 0.05),
    "cannon_enemy": lambda: pad(cannon(2, low=60, body=0.8, crack=0.8, seconds=1.2), 0.05),
    "turret_shot": turret_shot,
    "impact": lambda: explosion(5, 0.9, 0.6),
    "explosion": lambda: explosion(7, 2.4, 1.4),
    "hit_armor": lambda: clang(9, 310, 0.9, 0.25),
    "hit_enemy": lambda: clang(13, 420, 0.6, 0.14),
    "crate_break": crate_break,
    "reload": reload_click,
    "engine": lambda: engine_loop(17, 34),
    "engine_enemy": lambda: engine_loop(19, 42),
}


def write_wav(path, x):
    data = (np.clip(x, -1, 1) * 32767).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(data.tobytes())


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "assets/sounds"
    os.makedirs(out, exist_ok=True)
    for name, make in SOUNDS.items():
        x = make()
        write_wav(os.path.join(out, f"{name}.wav"), x)
        print(f"{name}.wav: {len(x) / RATE:.2f} s")


if __name__ == "__main__":
    main()
