//! 3D models made with 3dixel (art/*/model.py) and exported to GLB by tools/export_glb.py.

use crate::model_layout::{BARREL_PIVOT, MODEL_SCALE, TURRET_PIVOT, WHEELS, WheelKind};
use crate::render::with_transform;
use crate::tank::Tank;
use crate::turret_layout;
use crate::world::Enemy;
use raylib::ffi;
use raylib::prelude::*;
use std::path::PathBuf;

/// Finds a file under `assets/`, whether the game runs from the repo root or elsewhere.
pub fn asset_path(rel: &str) -> String {
    let local = PathBuf::from("assets").join(rel);
    if local.exists() {
        return local.to_string_lossy().into_owned();
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(rel)
        .to_string_lossy()
        .into_owned()
}

pub struct TankModels {
    hull: Model,
    turret: Model,
    barrel: Model,
    wheel_road: Model,
    wheel_end: Model,
}

pub struct TurretModels {
    base: Model,
    head: Model,
    gun: Model,
}

pub struct Models {
    pub tank: TankModels,
    pub turret: TurretModels,
    // Kept alive for as long as the models use it.
    shader: Shader,
    flash_loc: i32,
}

fn scale() -> Vector3 {
    Vector3::new(MODEL_SCALE, MODEL_SCALE, MODEL_SCALE)
}

impl Models {
    pub fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Result<Models, String> {
        let mut shader = rl.load_shader_from_memory(
            thread,
            Some(include_str!("../assets/shaders/lit.vs")),
            Some(include_str!("../assets/shaders/lit.fs")),
        );
        let emission = shader.get_shader_location("emissionMap");
        shader.locs_mut()[ffi::ShaderLocationIndex::SHADER_LOC_MAP_EMISSION as usize] = emission;
        let light = shader.get_shader_location("lightDir");
        shader.set_shader_value(light, Vector3::new(0.35, -1.0, -0.55).normalize());
        let flash_loc = shader.get_shader_location("flash");
        shader.set_shader_value(flash_loc, 0.0f32);
        let raw_shader: ffi::Shader = *shader.as_ref();

        let mut load = |path: &str| -> Result<Model, String> {
            let path = asset_path(path);
            let mut model = rl
                .load_model(thread, &path)
                .map_err(|e| format!("{path}: {e}"))?;
            for mat in model.materials_mut() {
                // SAFETY: we only swap the shader handle; raylib does not free shaders when
                // it unloads a model, and `shader` outlives every model in this struct.
                unsafe {
                    mat.as_raw_mut().shader = raw_shader;
                }
            }
            Ok(model)
        };
        let tank = TankModels {
            hull: load("models/tank/hull.glb")?,
            turret: load("models/tank/turret.glb")?,
            barrel: load("models/tank/barrel.glb")?,
            wheel_road: load("models/tank/wheel_road.glb")?,
            wheel_end: load("models/tank/wheel_end.glb")?,
        };
        let turret = TurretModels {
            base: load("models/turret/base.glb")?,
            head: load("models/turret/head.glb")?,
            gun: load("models/turret/gun.glb")?,
        };
        Ok(Models {
            tank,
            turret,
            shader,
            flash_loc,
        })
    }

    /// Hit flash: 0 draws normally, 1 draws the model white.
    fn set_flash(&mut self, v: f32) {
        self.shader.set_shader_value(self.flash_loc, v);
    }

    pub fn draw_tank<D: RaylibDraw3D>(&self, d: &mut D, t: &Tank) {
        let up = Vector3::new(0.0, 1.0, 0.0);
        let side = Vector3::new(1.0, 0.0, 0.0);
        let m = &self.tank;
        with_transform(t.pos, t.hull_yaw.to_degrees(), up, || {
            d.draw_model_ex(&m.hull, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
            for w in WHEELS.iter() {
                let model = match w.kind {
                    WheelKind::Road => &m.wheel_road,
                    WheelKind::End => &m.wheel_end,
                };
                with_transform(w.pos, t.wheel_angle(w.radius).to_degrees(), side, || {
                    d.draw_model_ex(model, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
                });
            }
            with_transform(TURRET_PIVOT, t.turret_yaw.to_degrees(), up, || {
                d.draw_model_ex(&m.turret, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
                with_transform(BARREL_PIVOT, -t.barrel_pitch.to_degrees(), side, || {
                    d.draw_model_ex(&m.barrel, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
                });
            });
        });
    }

    pub fn draw_enemy<D: RaylibDraw3D>(&mut self, d: &mut D, e: &Enemy) {
        let up = Vector3::new(0.0, 1.0, 0.0);
        let side = Vector3::new(1.0, 0.0, 0.0);
        let flash = if e.flash > 0.0 { 0.8 } else { 0.0 };
        if flash > 0.0 {
            self.set_flash(flash);
        }
        let m = &self.turret;
        with_transform(e.pos, 0.0, up, || {
            d.draw_model_ex(&m.base, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
            with_transform(turret_layout::HEAD_PIVOT, e.yaw.to_degrees(), up, || {
                d.draw_model_ex(&m.head, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
                with_transform(
                    turret_layout::GUN_PIVOT,
                    -e.pitch.to_degrees(),
                    side,
                    || {
                        d.draw_model_ex(&m.gun, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
                    },
                );
            });
        });
        if flash > 0.0 {
            self.set_flash(0.0);
        }
    }
}
