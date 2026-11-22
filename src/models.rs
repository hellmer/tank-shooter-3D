//! 3D models made with 3dixel (art/*/model.py) and exported to GLB by tools/export_glb.py,
//! plus the painted ground (art/terreno/ground.py).

use crate::enemy_tank::EnemyTank;
use crate::enemy_tank_layout;
use crate::model_layout::{BARREL_PIVOT, MODEL_SCALE, TURRET_PIVOT, WHEELS, Wheel, WheelKind};
use crate::props_layout;
use crate::render::with_transform;
use crate::tank::Tank;
use crate::turret_layout;
use crate::world::{ARENA_HALF, Enemy, Obstacle, ObstacleKind, Target};
use raylib::ffi;
use raylib::prelude::*;
use std::path::PathBuf;

/// Fog and sky horizon color, shared by the shaders and the sky gradient.
pub const FOG_COLOR: Color = Color {
    r: 168,
    g: 176,
    b: 178,
    a: 255,
};
pub const SKY_TOP: Color = Color {
    r: 92,
    g: 124,
    b: 156,
    a: 255,
};
const LIGHT_DIR: Vector3 = Vector3 {
    x: 0.35,
    y: -1.0,
    z: -0.55,
};

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

pub struct EnemyTankModels {
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

pub struct PropModels {
    crate_: Model,
    rock: Model,
    barrier: Model,
}

pub struct Models {
    pub tank: TankModels,
    pub turret: TurretModels,
    pub enemy_tank: EnemyTankModels,
    pub props: PropModels,
    ground: Model,
    // Kept alive for as long as the models use them.
    _ground_texture: Texture2D,
    ground_shader: Shader,
    shader: Shader,
    flash_loc: i32,
    view_locs: [i32; 2],
}

fn scale() -> Vector3 {
    Vector3::new(MODEL_SCALE, MODEL_SCALE, MODEL_SCALE)
}

fn color_vec(c: Color) -> Vector3 {
    Vector3::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0)
}

fn lit_shader(rl: &mut RaylibHandle, thread: &RaylibThread, fs: &str) -> Shader {
    let mut shader = rl.load_shader_from_memory(
        thread,
        Some(include_str!("../assets/shaders/lit.vs")),
        Some(fs),
    );
    let light = shader.get_shader_location("lightDir");
    shader.set_shader_value(light, LIGHT_DIR.normalize());
    let fog = shader.get_shader_location("fogColor");
    shader.set_shader_value(fog, color_vec(FOG_COLOR));
    shader
}

impl Models {
    pub fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Result<Models, String> {
        let mut shader = lit_shader(rl, thread, include_str!("../assets/shaders/lit.fs"));
        let emission = shader.get_shader_location("emissionMap");
        shader.locs_mut()[ffi::ShaderLocationIndex::SHADER_LOC_MAP_EMISSION as usize] = emission;
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
        let enemy_tank = EnemyTankModels {
            hull: load("models/enemy_tank/hull.glb")?,
            turret: load("models/enemy_tank/turret.glb")?,
            barrel: load("models/enemy_tank/barrel.glb")?,
            wheel_road: load("models/enemy_tank/wheel_road.glb")?,
            wheel_end: load("models/enemy_tank/wheel_end.glb")?,
        };
        let props = PropModels {
            crate_: load("models/props/crate.glb")?,
            rock: load("models/props/rock.glb")?,
            barrier: load("models/props/barrier.glb")?,
        };

        // Ground: a big plane whose texture coordinates come from the world position.
        let ground_shader = lit_shader(rl, thread, include_str!("../assets/shaders/ground.fs"));
        let tex_path = asset_path("textures/ground.png");
        let ground_texture = rl
            .load_texture(thread, &tex_path)
            .map_err(|e| format!("{tex_path}: {e}"))?;
        ground_texture.set_texture_wrap(thread, TextureWrap::TEXTURE_WRAP_REPEAT);
        ground_texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_POINT);
        let size = ARENA_HALF * 2.0 + 160.0;
        let mesh = Mesh::gen_mesh_plane(thread, size, size, 8, 8);
        // SAFETY: the model takes ownership of the mesh and unloads it with itself.
        let mut ground = rl
            .load_model_from_mesh(thread, unsafe { mesh.make_weak() })
            .map_err(|e| format!("ground: {e}"))?;
        for mat in ground.materials_mut() {
            mat.maps_mut()[ffi::MaterialMapIndex::MATERIAL_MAP_ALBEDO as usize].texture =
                *ground_texture.as_ref();
            // SAFETY: as above; `ground_shader` and `ground_texture` live as long as `ground`.
            unsafe {
                mat.as_raw_mut().shader = *ground_shader.as_ref();
            }
        }
        let view_locs = [
            shader.get_shader_location("viewPos"),
            ground_shader.get_shader_location("viewPos"),
        ];

        Ok(Models {
            tank,
            turret,
            enemy_tank,
            props,
            ground,
            _ground_texture: ground_texture,
            ground_shader,
            shader,
            flash_loc,
            view_locs,
        })
    }

    /// Tells the shaders where the camera is, for the fog.
    pub fn set_view(&mut self, pos: Vector3) {
        self.shader.set_shader_value(self.view_locs[0], pos);
        self.ground_shader.set_shader_value(self.view_locs[1], pos);
    }

    pub fn draw_ground<D: RaylibDraw3D>(&self, d: &mut D) {
        d.draw_model(&self.ground, Vector3::zero(), 1.0, Color::WHITE);
    }

    pub fn draw_obstacle<D: RaylibDraw3D>(&self, d: &mut D, o: &Obstacle) {
        let model = match o.kind {
            ObstacleKind::Rock => &self.props.rock,
            ObstacleKind::Barrier => &self.props.barrier,
        };
        let up = Vector3::new(0.0, 1.0, 0.0);
        d.draw_model_ex(
            model,
            o.pos,
            up,
            o.yaw().to_degrees(),
            scale(),
            Color::WHITE,
        );
    }

    /// The ring of barriers along the arena edge.
    pub fn draw_arena_edge<D: RaylibDraw3D>(&self, d: &mut D) {
        let up = Vector3::new(0.0, 1.0, 0.0);
        let step = props_layout::BARRIER_HALF.x * 2.0;
        let n = (ARENA_HALF * 2.0 / step).ceil() as i32;
        let edge = ARENA_HALF + props_layout::BARRIER_HALF.z;
        for i in 0..n {
            let t = -ARENA_HALF + step * (i as f32 + 0.5);
            for (pos, yaw) in [
                (Vector3::new(t, 0.0, edge), 0.0),
                (Vector3::new(t, 0.0, -edge), 180.0),
                (Vector3::new(edge, 0.0, t), 90.0),
                (Vector3::new(-edge, 0.0, t), 270.0),
            ] {
                d.draw_model_ex(&self.props.barrier, pos, up, yaw, scale(), Color::WHITE);
            }
        }
    }

    pub fn draw_crate<D: RaylibDraw3D>(&mut self, d: &mut D, t: &Target) {
        if t.flash > 0.0 {
            self.set_flash(0.8);
        }
        let base = Vector3::new(t.aabb.center.x, 0.0, t.aabb.center.z);
        let up = Vector3::new(0.0, 1.0, 0.0);
        d.draw_model_ex(
            &self.props.crate_,
            base,
            up,
            t.yaw().to_degrees(),
            scale(),
            Color::WHITE,
        );
        if t.flash > 0.0 {
            self.set_flash(0.0);
        }
    }

    /// Hit flash: 0 draws normally, 1 draws the model white.
    fn set_flash(&mut self, v: f32) {
        self.shader.set_shader_value(self.flash_loc, v);
    }

    pub fn draw_tank<D: RaylibDraw3D>(&self, d: &mut D, t: &Tank) {
        let m = &self.tank;
        let pose = TankPose {
            pos: t.pos,
            hull_yaw: t.hull_yaw,
            turret_yaw: t.turret_yaw,
            barrel_pitch: t.barrel_pitch,
            track_offset: t.track_offset,
        };
        draw_tracked(
            d,
            [&m.hull, &m.turret, &m.barrel, &m.wheel_road, &m.wheel_end],
            &pose,
            &WHEELS,
            TURRET_PIVOT,
            BARREL_PIVOT,
        );
    }

    pub fn draw_enemy_tank<D: RaylibDraw3D>(&mut self, d: &mut D, t: &EnemyTank) {
        if t.flash > 0.0 {
            self.set_flash(0.8);
        }
        let m = &self.enemy_tank;
        let hull = TankPose {
            pos: t.pos,
            hull_yaw: t.hull_yaw,
            turret_yaw: t.turret_rel_yaw(),
            barrel_pitch: t.barrel_pitch,
            track_offset: t.track_offset,
        };
        draw_tracked(
            d,
            [&m.hull, &m.turret, &m.barrel, &m.wheel_road, &m.wheel_end],
            &hull,
            &enemy_tank_layout::WHEELS,
            enemy_tank_layout::TURRET_PIVOT,
            enemy_tank_layout::BARREL_PIVOT,
        );
        if t.flash > 0.0 {
            self.set_flash(0.0);
        }
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

/// Where a tracked vehicle is and how its moving parts are turned.
struct TankPose {
    pos: Vector3,
    hull_yaw: f32,
    /// Relative to the hull.
    turret_yaw: f32,
    barrel_pitch: f32,
    /// Distance driven, for the wheels.
    track_offset: f32,
}

/// Draws a tank made of hull, turret, barrel and spinning wheels (the player's or an enemy's).
/// `parts` is hull, turret, barrel, road wheel, end wheel.
fn draw_tracked<D: RaylibDraw3D>(
    d: &mut D,
    parts: [&Model; 5],
    t: &TankPose,
    wheels: &[Wheel],
    turret_pivot: Vector3,
    barrel_pivot: Vector3,
) {
    let up = Vector3::new(0.0, 1.0, 0.0);
    let side = Vector3::new(1.0, 0.0, 0.0);
    let [hull, turret, barrel, road, end] = parts;
    with_transform(t.pos, t.hull_yaw.to_degrees(), up, || {
        d.draw_model_ex(hull, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
        for w in wheels {
            let model = match w.kind {
                WheelKind::Road => road,
                WheelKind::End => end,
            };
            // Positive rolls forward.
            let angle = (t.track_offset / w.radius).to_degrees();
            with_transform(w.pos, angle, side, || {
                d.draw_model_ex(model, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
            });
        }
        with_transform(turret_pivot, t.turret_yaw.to_degrees(), up, || {
            d.draw_model_ex(turret, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
            with_transform(barrel_pivot, -t.barrel_pitch.to_degrees(), side, || {
                d.draw_model_ex(barrel, Vector3::zero(), up, 0.0, scale(), Color::WHITE);
            });
        });
    });
}
