//! The tank built with 3dixel (art/tanque/model.py), exported to GLB by tools/export_glb.py.

use crate::model_layout::{BARREL_PIVOT, MODEL_SCALE, TURRET_PIVOT, WHEELS, WheelKind};
use crate::render::with_transform;
use crate::tank::Tank;
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
    // Kept alive for as long as the models use it.
    _shader: Shader,
}

impl TankModels {
    pub fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Result<TankModels, String> {
        let mut shader = rl.load_shader_from_memory(
            thread,
            Some(include_str!("../assets/shaders/lit.vs")),
            Some(include_str!("../assets/shaders/lit.fs")),
        );
        let emission = shader.get_shader_location("emissionMap");
        shader.locs_mut()[ffi::ShaderLocationIndex::SHADER_LOC_MAP_EMISSION as usize] = emission;
        let light = shader.get_shader_location("lightDir");
        let dir = Vector3::new(0.35, -1.0, -0.55).normalize();
        shader.set_shader_value(light, dir);
        let raw_shader: ffi::Shader = *shader.as_ref();

        let mut load = |name: &str| -> Result<Model, String> {
            let path = asset_path(&format!("models/tank/{name}.glb"));
            let mut model = rl
                .load_model(thread, &path)
                .map_err(|e| format!("{path}: {e}"))?;
            for mat in model.materials_mut() {
                // SAFETY: we only swap the shader handle; raylib does not free shaders when
                // it unloads a model, and `_shader` outlives every model in this struct.
                unsafe {
                    mat.as_raw_mut().shader = raw_shader;
                }
            }
            Ok(model)
        };
        Ok(TankModels {
            hull: load("hull")?,
            turret: load("turret")?,
            barrel: load("barrel")?,
            wheel_road: load("wheel_road")?,
            wheel_end: load("wheel_end")?,
            _shader: shader,
        })
    }

    pub fn draw<D: RaylibDraw3D>(&self, d: &mut D, t: &Tank) {
        let up = Vector3::new(0.0, 1.0, 0.0);
        let side = Vector3::new(1.0, 0.0, 0.0);
        let s = Vector3::new(MODEL_SCALE, MODEL_SCALE, MODEL_SCALE);
        with_transform(t.pos, t.hull_yaw.to_degrees(), up, || {
            d.draw_model_ex(&self.hull, Vector3::zero(), up, 0.0, s, Color::WHITE);
            for w in WHEELS.iter() {
                let model = match w.kind {
                    WheelKind::Road => &self.wheel_road,
                    WheelKind::End => &self.wheel_end,
                };
                with_transform(w.pos, t.wheel_angle(w.radius).to_degrees(), side, || {
                    d.draw_model_ex(model, Vector3::zero(), up, 0.0, s, Color::WHITE);
                });
            }
            with_transform(TURRET_PIVOT, t.turret_yaw.to_degrees(), up, || {
                d.draw_model_ex(&self.turret, Vector3::zero(), up, 0.0, s, Color::WHITE);
                with_transform(BARREL_PIVOT, -t.barrel_pitch.to_degrees(), side, || {
                    d.draw_model_ex(&self.barrel, Vector3::zero(), up, 0.0, s, Color::WHITE);
                });
            });
        });
    }
}
