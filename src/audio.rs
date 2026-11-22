//! Sound: plays the synthesized effects (`tools/sfx.py`, in `assets/sounds/`) for the world's
//! sound events, quieter with distance and panned left or right from the camera's point of view,
//! plus a looping engine for the player's tank and one for each enemy tank, and the music
//! (`tools/music.py`): a calm loop that always plays and a combat loop over it that fades in
//! when enemies are near.

use crate::enemy_tank;
use crate::models::asset_path;
use crate::tank;
use crate::world::{Sfx, World};
use raylib::ffi;
use raylib::prelude::Vector3;
use std::ffi::CString;

/// How many copies of each effect can sound at the same time.
const VOICES: usize = 4;
/// Music level, under the effects.
const MUSIC_LEVEL: f32 = 0.32;
/// Overall level of the effects, leaving headroom for several at once.
const EFFECTS_LEVEL: f32 = 0.7;

const ALL: [Sfx; 9] = [
    Sfx::PlayerCannon,
    Sfx::EnemyCannon,
    Sfx::TurretShot,
    Sfx::Impact,
    Sfx::Explosion,
    Sfx::HitArmor,
    Sfx::HitEnemy,
    Sfx::CrateBreak,
    Sfx::Reload,
];

fn file(sfx: Sfx) -> &'static str {
    match sfx {
        Sfx::PlayerCannon => "cannon_player",
        Sfx::EnemyCannon => "cannon_enemy",
        Sfx::TurretShot => "turret_shot",
        Sfx::Impact => "impact",
        Sfx::Explosion => "explosion",
        Sfx::HitArmor => "hit_armor",
        Sfx::HitEnemy => "hit_enemy",
        Sfx::CrateBreak => "crate_break",
        Sfx::Reload => "reload",
    }
}

/// Loudness right next to the listener, and the distance (m) at which it has dropped to half.
fn mix(sfx: Sfx) -> (f32, f32) {
    match sfx {
        Sfx::PlayerCannon => (0.9, 40.0),
        Sfx::EnemyCannon => (0.85, 30.0),
        Sfx::TurretShot => (0.6, 25.0),
        Sfx::Impact => (0.6, 18.0),
        Sfx::Explosion => (1.0, 35.0),
        Sfx::HitArmor => (0.9, 40.0),
        Sfx::HitEnemy => (0.55, 25.0),
        Sfx::CrateBreak => (0.6, 18.0),
        Sfx::Reload => (0.35, 40.0),
    }
}

fn falloff(distance: f32, half: f32) -> f32 {
    1.0 / (1.0 + (distance / half).powi(2))
}

struct Effect {
    base: ffi::Sound,
    voices: Vec<ffi::Sound>,
    next: usize,
}

/// Where the player hears from: the tank's position, facing the way the camera looks.
struct Listener {
    pos: Vector3,
    right: Vector3,
}

impl Listener {
    /// Volume factor and stereo pan (-1 left .. 1 right) for a sound at `p`.
    fn place(&self, p: Vector3, half: f32) -> (f32, f32) {
        let to = Vector3::new(p.x - self.pos.x, 0.0, p.z - self.pos.z);
        let d = to.length();
        let pan = if d > 0.5 {
            (to.dot(self.right) / d) * (d / (d + 4.0))
        } else {
            0.0
        };
        (falloff(d, half), pan.clamp(-1.0, 1.0) * 0.8)
    }
}

pub struct Audio {
    effects: Vec<Effect>,
    engine: Option<ffi::Music>,
    enemy_engines: Vec<ffi::Music>,
    /// Calm and combat loops, started together so they stay in time.
    music: Option<(ffi::Music, ffi::Music)>,
    /// How loud the combat loop is right now (0..1), following `World::danger` slowly.
    combat: f32,
    /// Player's volume settings (0..1).
    effects_volume: f32,
    music_volume: f32,
    muted: bool,
    /// For small pitch changes, so repeated shots do not sound identical.
    jitter: u32,
}

fn cpath(rel: &str) -> CString {
    CString::new(asset_path(rel)).expect("asset path has no NUL bytes")
}

fn load_music(rel: &str) -> Option<ffi::Music> {
    let path = cpath(rel);
    // SAFETY: plain raylib calls with a valid C string; the stream is unloaded in `Drop`.
    unsafe {
        let mut m = ffi::LoadMusicStream(path.as_ptr());
        if !ffi::IsMusicValid(m) {
            return None;
        }
        m.looping = true;
        ffi::SetMusicVolume(m, 0.0);
        ffi::PlayMusicStream(m);
        Some(m)
    }
}

impl Audio {
    /// Opens the audio device and loads every sound. None when there is no audio device;
    /// missing sound files are skipped with a warning and the game stays playable.
    pub fn new() -> Option<Audio> {
        // SAFETY: raylib's audio device is opened once here and closed in `Drop`.
        unsafe {
            ffi::InitAudioDevice();
            if !ffi::IsAudioDeviceReady() {
                eprintln!("No audio device: playing without sound");
                return None;
            }
        }
        let mut effects = Vec::new();
        for sfx in ALL {
            let path = cpath(&format!("sounds/{}.wav", file(sfx)));
            // SAFETY: as above; every sound and alias is unloaded in `Drop`.
            let effect = unsafe {
                let base = ffi::LoadSound(path.as_ptr());
                if !ffi::IsSoundValid(base) {
                    eprintln!("Missing sound {}: run tools/sfx.py", file(sfx));
                }
                let voices = (0..VOICES).map(|_| ffi::LoadSoundAlias(base)).collect();
                Effect {
                    base,
                    voices,
                    next: 0,
                }
            };
            effects.push(effect);
        }
        let engine = load_music("sounds/engine.wav");
        let enemy_engines = (0..crate::world::ENEMY_TANK_COUNT)
            .filter_map(|_| load_music("sounds/engine_enemy.wav"))
            .collect();
        let music = match (load_music("music/calm.ogg"), load_music("music/combat.ogg")) {
            (Some(calm), Some(combat)) => Some((calm, combat)),
            (calm, combat) => {
                eprintln!("Missing music: run tools/music.py");
                // SAFETY: unload whichever of the two did load.
                unsafe {
                    for m in calm.iter().chain(combat.iter()) {
                        ffi::UnloadMusicStream(*m);
                    }
                }
                None
            }
        };
        Some(Audio {
            effects,
            engine,
            enemy_engines,
            music,
            combat: 0.0,
            effects_volume: 1.0,
            music_volume: 1.0,
            muted: false,
            jitter: 1,
        })
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        // SAFETY: the device is open while `self` lives.
        unsafe { ffi::SetMasterVolume(if self.muted { 0.0 } else { 1.0 }) }
    }

    pub fn muted(&self) -> bool {
        self.muted
    }

    /// Applies the player's volume options.
    pub fn set_levels(&mut self, settings: &crate::settings::Settings) {
        self.effects_volume = settings.effects_volume;
        self.music_volume = settings.music_volume;
    }

    fn next_jitter(&mut self) -> f32 {
        self.jitter = self
            .jitter
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.jitter >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Plays this frame's events and keeps the engines and music running. `eye` and `target` are
    /// the camera's. `playing` is false in the menus: no effects or engines, calm music only.
    pub fn update(&mut self, w: &World, eye: Vector3, target: Vector3, playing: bool, dt: f32) {
        let look = Vector3::new(target.x - eye.x, 0.0, target.z - eye.z).normalize();
        // Right of the view: look × up (raylib is right-handed with +Y up).
        let right = Vector3::new(-look.z, 0.0, look.x);
        let ear = Listener {
            pos: w.tank.pos,
            right,
        };

        let events = if playing { &w.sounds[..] } else { &[] };
        let fx_level = EFFECTS_LEVEL * self.effects_volume;
        // Engines are effects too, and stay quiet in the menus.
        let engine_level = if playing { self.effects_volume } else { 0.0 };
        for ev in events {
            let (gain, half) = mix(ev.sfx);
            let (near, pan) = ear.place(ev.pos, half);
            let volume = gain * near * fx_level;
            if volume < 0.02 {
                continue;
            }
            let pitch = 0.93 + 0.14 * self.next_jitter();
            let fx = &mut self.effects[ALL.iter().position(|s| *s == ev.sfx).unwrap_or(0)];
            let voice = fx.voices[fx.next];
            fx.next = (fx.next + 1) % fx.voices.len();
            // SAFETY: `voice` is a live alias owned by `self`.
            unsafe {
                ffi::SetSoundVolume(voice, volume);
                ffi::SetSoundPan(voice, pan);
                ffi::SetSoundPitch(voice, pitch);
                ffi::PlaySound(voice);
            }
        }

        // Music: the combat layer swells quickly when a fight starts and fades out slowly after.
        let want = if playing { w.danger() } else { 0.0 };
        let rate = if want > self.combat { 0.5 } else { 0.12 };
        self.combat = crate::math::approach(self.combat, want, rate * dt);
        if let Some((calm, combat)) = self.music {
            // SAFETY: both streams are live and owned by `self`.
            unsafe {
                let level = MUSIC_LEVEL * self.music_volume;
                ffi::SetMusicVolume(calm, level * (1.0 - 0.3 * self.combat));
                ffi::SetMusicVolume(combat, level * 1.1 * self.combat);
                ffi::UpdateMusicStream(calm);
                ffi::UpdateMusicStream(combat);
            }
        }

        // Engines: pitch rises with speed; the player's is heard from inside, the enemies' by distance.
        if let Some(m) = self.engine {
            let load = (w.tank.speed.abs() / tank::MAX_SPEED).min(1.0);
            let volume = if w.player_alive() {
                0.22 + 0.2 * load
            } else {
                0.0
            };
            // SAFETY: `m` is a live stream owned by `self`.
            unsafe {
                ffi::SetMusicVolume(m, volume * engine_level);
                ffi::SetMusicPitch(m, 0.8 + 0.55 * load);
                ffi::UpdateMusicStream(m);
            }
        }
        for (k, m) in self.enemy_engines.iter().enumerate() {
            let (volume, pan, pitch) = match w.enemy_tanks.get(k).filter(|t| t.alive()) {
                Some(t) => {
                    let (near, pan) = ear.place(t.pos, 14.0);
                    let load = (t.speed.abs() / enemy_tank::MAX_SPEED).min(1.0);
                    (0.5 * near, pan, 0.85 + 0.45 * load)
                }
                None => (0.0, 0.0, 1.0),
            };
            // SAFETY: as above.
            unsafe {
                ffi::SetMusicVolume(*m, volume * engine_level);
                ffi::SetMusicPan(*m, pan);
                ffi::SetMusicPitch(*m, pitch);
                ffi::UpdateMusicStream(*m);
            }
        }
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        // SAFETY: everything here was loaded in `new` and is unloaded exactly once.
        unsafe {
            for fx in &self.effects {
                for v in &fx.voices {
                    ffi::UnloadSoundAlias(*v);
                }
                ffi::UnloadSound(fx.base);
            }
            let music = self.music.iter().flat_map(|(a, b)| [a, b]);
            for m in self
                .engine
                .iter()
                .chain(self.enemy_engines.iter())
                .chain(music)
            {
                ffi::UnloadMusicStream(*m);
            }
            ffi::CloseAudioDevice();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_sounds_are_quieter_and_side_sounds_pan() {
        let ear = Listener {
            pos: Vector3::zero(),
            // Looking along +Z, so right is -X (yaw convention: +X is to the left).
            right: Vector3::new(-1.0, 0.0, 0.0),
        };
        let (near, _) = ear.place(Vector3::new(0.0, 0.0, 5.0), 20.0);
        let (far, _) = ear.place(Vector3::new(0.0, 0.0, 80.0), 20.0);
        assert!(near > far * 5.0);
        let (_, right) = ear.place(Vector3::new(-30.0, 0.0, 0.0), 20.0);
        let (_, left) = ear.place(Vector3::new(30.0, 0.0, 0.0), 20.0);
        assert!(right > 0.5 && left < -0.5);
    }

    #[test]
    fn every_effect_has_a_file() {
        for sfx in ALL {
            let path = asset_path(&format!("sounds/{}.wav", file(sfx)));
            assert!(std::path::Path::new(&path).exists(), "{path}");
        }
        assert!(std::path::Path::new(&asset_path("sounds/engine.wav")).exists());
        assert!(std::path::Path::new(&asset_path("sounds/engine_enemy.wav")).exists());
        assert!(std::path::Path::new(&asset_path("music/calm.ogg")).exists());
        assert!(std::path::Path::new(&asset_path("music/combat.ogg")).exists());
    }
}
