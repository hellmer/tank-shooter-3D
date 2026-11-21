//! Drawing. The tank is the 3dixel model; the box tank is only a fallback if the model files are missing.

use crate::tank::{self, Tank};
use crate::tank_model::TankModels;
use crate::world::{ARENA_HALF, World};
use raylib::ffi;
use raylib::prelude::*;

const HULL: Color = Color {
    r: 46,
    g: 50,
    b: 56,
    a: 255,
};
const HULL_DARK: Color = Color {
    r: 33,
    g: 36,
    b: 41,
    a: 255,
};
const TRACK: Color = Color {
    r: 24,
    g: 25,
    b: 27,
    a: 255,
};
const WHEEL: Color = Color {
    r: 70,
    g: 70,
    b: 62,
    a: 255,
};
const CYAN: Color = Color {
    r: 70,
    g: 235,
    b: 255,
    a: 255,
};
const ORANGE: Color = Color {
    r: 255,
    g: 160,
    b: 70,
    a: 255,
};
const GROUND: Color = Color {
    r: 58,
    g: 62,
    b: 58,
    a: 255,
};

/// Runs `f` with the model matrix translated and rotated (degrees around an axis).
pub fn with_transform(pos: Vector3, angle_deg: f32, axis: Vector3, f: impl FnOnce()) {
    unsafe {
        ffi::rlPushMatrix();
        ffi::rlTranslatef(pos.x, pos.y, pos.z);
        ffi::rlRotatef(angle_deg, axis.x, axis.y, axis.z);
    }
    f();
    unsafe {
        ffi::rlPopMatrix();
    }
}

fn cube<D: RaylibDraw3D>(d: &mut D, center: (f32, f32, f32), size: (f32, f32, f32), color: Color) {
    d.draw_cube(
        Vector3::new(center.0, center.1, center.2),
        size.0,
        size.1,
        size.2,
        color,
    );
}

/// Box with darker outline so the shapes read at a distance.
fn block<D: RaylibDraw3D>(d: &mut D, center: (f32, f32, f32), size: (f32, f32, f32), color: Color) {
    let c = Vector3::new(center.0, center.1, center.2);
    d.draw_cube(c, size.0, size.1, size.2, color);
    d.draw_cube_wires(c, size.0, size.1, size.2, Color::new(15, 16, 18, 255));
}

pub fn draw_tank<D: RaylibDraw3D>(d: &mut D, t: &Tank) {
    let up = Vector3::new(0.0, 1.0, 0.0);
    with_transform(t.pos, t.hull_yaw.to_degrees(), up, || {
        // Tracks with road wheels; the wheel bands slide with the distance driven.
        for side in [-1.0f32, 1.0] {
            let x = side * 1.45;
            block(d, (x, 0.5, 0.0), (0.8, 1.0, 6.6), TRACK);
            for i in 0..6 {
                let z = -2.5 + i as f32 * 1.0;
                d.draw_cylinder_ex(
                    Vector3::new(x + side * 0.41, 0.45, z),
                    Vector3::new(x + side * 0.43, 0.45, z),
                    0.38,
                    0.38,
                    10,
                    WHEEL,
                );
            }
            let phase = t.track_offset.rem_euclid(0.6);
            for i in 0..11 {
                let z = -3.3 + phase + i as f32 * 0.6;
                if z < 3.3 {
                    cube(d, (x, 1.01, z), (0.82, 0.02, 0.12), CYAN);
                }
            }
        }
        // Hull body, with a sloped-looking nose made of a lower front block.
        block(d, (0.0, 1.05, -0.2), (2.4, 0.9, 6.0), HULL);
        block(d, (0.0, 0.85, 3.0), (2.4, 0.5, 0.8), HULL_DARK);
        // Cyan side strips and rear engine grille.
        for side in [-1.0f32, 1.0] {
            for i in 0..4 {
                cube(
                    d,
                    (side * 1.86, 0.95, -1.8 + i as f32 * 1.2),
                    (0.03, 0.1, 0.5),
                    CYAN,
                );
            }
        }
        cube(d, (0.0, 1.51, -2.6), (1.6, 0.02, 0.8), CYAN);
        // Rear thrusters.
        for side in [-0.6f32, 0.6] {
            cube(d, (side, 1.0, -3.25), (0.4, 0.4, 0.1), ORANGE);
        }

        // Turret and barrel.
        with_transform(tank::TURRET_PIVOT, t.turret_yaw.to_degrees(), up, || {
            block(d, (0.0, 0.4, 0.1), (2.3, 0.8, 3.0), HULL);
            block(d, (0.0, 0.5, -1.7), (1.8, 0.6, 0.8), HULL_DARK);
            cube(d, (-0.9, 0.55, 1.61), (0.4, 0.06, 0.02), CYAN);
            cube(d, (0.9, 0.55, 1.61), (0.4, 0.06, 0.02), CYAN);
            // Sensor pod on top.
            block(d, (0.6, 0.95, -0.4), (0.5, 0.35, 0.5), HULL_DARK);
            cube(
                d,
                (0.6, 0.95, -0.14),
                (0.3, 0.12, 0.02),
                Color::new(255, 70, 70, 255),
            );

            with_transform(
                tank::BARREL_PIVOT,
                -t.barrel_pitch.to_degrees(),
                Vector3::new(1.0, 0.0, 0.0),
                || {
                    d.draw_cylinder_ex(
                        Vector3::zero(),
                        Vector3::new(0.0, 0.0, tank::BARREL_LENGTH),
                        0.17,
                        0.15,
                        12,
                        HULL_DARK,
                    );
                    d.draw_cylinder_ex(
                        Vector3::new(0.0, 0.0, 0.3),
                        Vector3::new(0.0, 0.0, 0.9),
                        0.24,
                        0.24,
                        12,
                        HULL,
                    );
                    d.draw_cylinder_ex(
                        Vector3::new(0.0, 0.0, tank::BARREL_LENGTH - 0.4),
                        Vector3::new(0.0, 0.0, tank::BARREL_LENGTH),
                        0.22,
                        0.22,
                        12,
                        HULL,
                    );
                },
            );
        });
    });
}

pub fn draw_world<D: RaylibDraw3D>(d: &mut D, w: &World, models: Option<&TankModels>) {
    // Ground and arena edge.
    d.draw_plane(
        Vector3::zero(),
        Vector2::new(ARENA_HALF * 2.0, ARENA_HALF * 2.0),
        GROUND,
    );
    // Lifted a little so the grid does not flicker against the ground plane.
    with_transform(
        Vector3::new(0.0, 0.03, 0.0),
        0.0,
        Vector3::new(0.0, 1.0, 0.0),
        || d.draw_grid(90, 2.0),
    );
    for (x, z, sx, sz) in [
        (0.0, ARENA_HALF, ARENA_HALF * 2.0, 1.0),
        (0.0, -ARENA_HALF, ARENA_HALF * 2.0, 1.0),
        (ARENA_HALF, 0.0, 1.0, ARENA_HALF * 2.0),
        (-ARENA_HALF, 0.0, 1.0, ARENA_HALF * 2.0),
    ] {
        block(d, (x, 1.0, z), (sx, 2.0, sz), Color::new(80, 84, 90, 255));
    }

    for t in w.targets.iter().filter(|t| t.alive()) {
        let c = t.aabb.center;
        let s = t.aabb.half * 2.0;
        let base = match t.hp {
            3 => Color::new(160, 120, 70, 255),
            2 => Color::new(170, 95, 55, 255),
            _ => Color::new(175, 65, 45, 255),
        };
        let color = if t.flash > 0.0 { Color::WHITE } else { base };
        block(d, (c.x, c.y, c.z), (s.x, s.y, s.z), color);
        // Bullseye stripe so targets read as targets.
        cube(
            d,
            (c.x, c.y + t.aabb.half.y - 0.3, c.z),
            (s.x + 0.02, 0.2, s.z + 0.02),
            Color::new(235, 225, 200, 255),
        );
    }

    match models {
        Some(m) => m.draw(d, &w.tank),
        None => draw_tank(d, &w.tank),
    }

    for s in &w.shells {
        d.draw_sphere(s.pos, 0.18, ORANGE);
        let tail = s.pos - s.vel * 0.02;
        d.draw_cylinder_ex(tail, s.pos, 0.02, 0.12, 6, Color::new(255, 200, 120, 160));
    }

    for e in &w.explosions {
        let p = e.progress();
        let r = e.radius * (0.3 + 0.7 * p);
        let a = ((1.0 - p) * 220.0) as u8;
        d.draw_sphere(e.pos, r, Color::new(255, 150, 50, a));
        d.draw_sphere(e.pos, r * 0.6, Color::new(255, 235, 150, a));
    }
}

pub fn camera(w: &World) -> Camera3D {
    let (eye, target) = w.camera_pose();
    Camera3D::perspective(eye, target, Vector3::new(0.0, 1.0, 0.0), 60.0)
}

pub fn draw_hud(d: &mut RaylibDrawHandle, w: &World, cam: Camera3D, gamepad: bool) {
    let sw = d.get_screen_width();
    let sh = d.get_screen_height();
    let (cx, cy) = (sw / 2, sh / 2);

    // Crosshair where the camera aims.
    let white = Color::new(240, 240, 240, 220);
    d.draw_line(cx - 12, cy, cx - 4, cy, white);
    d.draw_line(cx + 4, cy, cx + 12, cy, white);
    d.draw_line(cx, cy - 12, cx, cy - 4, white);
    d.draw_line(cx, cy + 4, cx, cy + 12, white);

    // Where the barrel actually points right now (lags while the turret turns).
    let barrel_hit = w.raycast(w.tank.muzzle(), w.tank.barrel_dir());
    let p = d.get_world_to_screen(barrel_hit, cam);
    let ready = w.tank.can_fire();
    let ring = if ready {
        Color::new(70, 235, 255, 230)
    } else {
        Color::new(255, 160, 70, 230)
    };
    d.draw_circle_lines(p.x as i32, p.y as i32, 9.0, ring);

    // Reload bar.
    let frac = 1.0 - w.tank.reload / tank::RELOAD_TIME;
    d.draw_rectangle(cx - 40, cy + 26, 80, 5, Color::new(0, 0, 0, 120));
    d.draw_rectangle(cx - 40, cy + 26, (80.0 * frac) as i32, 5, ring);

    d.draw_text(
        &format!("Blancos destruidos: {}", w.score),
        20,
        20,
        24,
        Color::RAYWHITE,
    );
    let help = if gamepad {
        "Mando: stick izq. mover | stick der. apuntar | RT disparar"
    } else {
        "WASD mover | raton apuntar | clic izq. / espacio disparar | Esc salir"
    };
    d.draw_text(help, 20, sh - 34, 20, Color::new(220, 220, 220, 200));
    d.draw_fps(sw - 100, 20);
}
