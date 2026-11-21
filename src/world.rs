//! Game state: the tank, targets, shells and effects, and how they interact.

use crate::input::Controls;
use crate::math::{Aabb, Rng, dir_from_angles};
use crate::tank::{self, Tank};
use raylib::prelude::Vector3;

pub const ARENA_HALF: f32 = 90.0;
pub const SHELL_SPEED: f32 = 80.0;
pub const SHELL_LIFE: f32 = 3.0;
pub const MAX_AIM_RANGE: f32 = 250.0;
pub const TARGET_COUNT: usize = 12;
pub const TARGET_HP: i32 = 3;
pub const TARGET_RESPAWN: f32 = 4.0;

pub struct Target {
    pub aabb: Aabb,
    pub hp: i32,
    /// Seconds of hit flash left.
    pub flash: f32,
    /// Seconds until the target comes back, when destroyed.
    pub respawn: Option<f32>,
}

impl Target {
    pub fn alive(&self) -> bool {
        self.respawn.is_none()
    }
}

pub struct Shell {
    pub pos: Vector3,
    pub vel: Vector3,
    pub life: f32,
}

pub struct Explosion {
    pub pos: Vector3,
    pub age: f32,
    pub duration: f32,
    pub radius: f32,
}

impl Explosion {
    pub fn progress(&self) -> f32 {
        (self.age / self.duration).min(1.0)
    }
}

pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl OrbitCamera {
    pub const MIN_PITCH: f32 = -0.45;
    pub const MAX_PITCH: f32 = 0.5;

    pub fn dir(&self) -> Vector3 {
        dir_from_angles(self.yaw, self.pitch)
    }
}

pub struct World {
    pub tank: Tank,
    pub camera: OrbitCamera,
    pub targets: Vec<Target>,
    pub shells: Vec<Shell>,
    pub explosions: Vec<Explosion>,
    pub score: u32,
    pub aim_point: Vector3,
    rng: Rng,
}

impl World {
    pub fn new(seed: u64) -> World {
        let mut w = World {
            tank: Tank::new(Vector3::zero()),
            camera: OrbitCamera {
                yaw: 0.0,
                pitch: -0.1,
                distance: 13.0,
            },
            targets: Vec::new(),
            shells: Vec::new(),
            explosions: Vec::new(),
            score: 0,
            aim_point: Vector3::new(0.0, 0.0, 50.0),
            rng: Rng::new(seed),
        };
        for _ in 0..TARGET_COUNT {
            let aabb = w.random_target_box();
            w.targets.push(Target {
                aabb,
                hp: TARGET_HP,
                flash: 0.0,
                respawn: None,
            });
        }
        w
    }

    fn random_target_box(&mut self) -> Aabb {
        loop {
            let angle = self.rng.range(0.0, std::f32::consts::TAU);
            let dist = self.rng.range(18.0, 70.0);
            let half = Vector3::new(
                self.rng.range(1.0, 2.2),
                self.rng.range(1.0, 2.5),
                self.rng.range(1.0, 2.2),
            );
            let center = Vector3::new(angle.sin() * dist, half.y, angle.cos() * dist);
            let clear_of_tank = center.distance(self.tank.pos) > 12.0;
            let clear_of_others = self
                .targets
                .iter()
                .all(|t| t.aabb.center.distance(center) > 7.0);
            if clear_of_tank && clear_of_others {
                return Aabb { center, half };
            }
        }
    }

    /// Where the camera's center ray first hits the ground or a target.
    pub fn raycast(&self, origin: Vector3, dir: Vector3) -> Vector3 {
        let mut best = MAX_AIM_RANGE;
        if dir.y < -1e-4 {
            best = best.min(-origin.y / dir.y);
        }
        for t in self.targets.iter().filter(|t| t.alive()) {
            if let Some(d) = t.aabb.ray_hit(origin, dir) {
                best = best.min(d);
            }
        }
        origin + dir * best
    }

    /// Camera position and look target for this frame.
    pub fn camera_pose(&self) -> (Vector3, Vector3) {
        let focus = self.tank.pos + Vector3::new(0.0, 4.2, 0.0);
        let dir = self.camera.dir();
        let mut eye = focus - dir * self.camera.distance;
        eye.y = eye.y.max(0.6);
        (eye, focus + dir * 10.0)
    }

    pub fn update(&mut self, c: &Controls, dt: f32) {
        // Camera.
        self.camera.yaw += c.look_yaw;
        self.camera.pitch = (self.camera.pitch + c.look_pitch)
            .clamp(OrbitCamera::MIN_PITCH, OrbitCamera::MAX_PITCH);

        // Tank movement and collisions.
        self.tank.drive(c, dt);
        for t in self.targets.iter().filter(|t| t.alive()) {
            let pushed = t
                .aabb
                .push_circle_out(self.tank.pos, tank::COLLISION_RADIUS);
            if pushed.x != self.tank.pos.x || pushed.z != self.tank.pos.z {
                self.tank.speed *= 0.5;
            }
            self.tank.pos = pushed;
        }
        let lim = ARENA_HALF - tank::COLLISION_RADIUS;
        self.tank.pos.x = self.tank.pos.x.clamp(-lim, lim);
        self.tank.pos.z = self.tank.pos.z.clamp(-lim, lim);

        // Aim: the turret follows whatever the crosshair points at.
        let (eye, _) = self.camera_pose();
        self.aim_point = self.raycast(eye, self.camera.dir());
        self.tank.aim_at(self.aim_point, dt);

        // Fire.
        if c.fire && self.tank.can_fire() {
            self.tank.reload = tank::RELOAD_TIME;
            let muzzle = self.tank.muzzle();
            self.shells.push(Shell {
                pos: muzzle,
                vel: self.tank.barrel_dir() * SHELL_SPEED,
                life: SHELL_LIFE,
            });
            self.explosions.push(Explosion {
                pos: muzzle,
                age: 0.0,
                duration: 0.12,
                radius: 0.7,
            });
            // A little recoil.
            self.tank.speed -= 1.5;
        }

        self.update_shells(dt);
        self.update_targets(dt);

        for e in &mut self.explosions {
            e.age += dt;
        }
        self.explosions.retain(|e| e.age < e.duration);
    }

    fn update_shells(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.shells.len() {
            let s = &mut self.shells[i];
            let step = s.vel * dt;
            let start = s.pos;
            s.pos += step;
            s.life -= dt;
            let len = step.length();
            let dir = if len > 0.0 { step * (1.0 / len) } else { step };

            // Swept hit test so fast shells do not pass through thin boxes.
            let mut hit: Option<(f32, Option<usize>)> = None;
            if s.pos.y <= 0.0 && dir.y < 0.0 {
                hit = Some((-start.y / dir.y, None));
            }
            for (ti, t) in self.targets.iter().enumerate().filter(|(_, t)| t.alive()) {
                if let Some(d) = t.aabb.ray_hit(start, dir)
                    && d <= len
                    && hit.is_none_or(|(best, _)| d < best)
                {
                    hit = Some((d, Some(ti)));
                }
            }

            let out_of_arena = s.pos.x.abs() > ARENA_HALF || s.pos.z.abs() > ARENA_HALF;
            if let Some((d, target)) = hit {
                let at = start + dir * d;
                self.explosions.push(Explosion {
                    pos: at,
                    age: 0.0,
                    duration: 0.45,
                    radius: 2.2,
                });
                if let Some(ti) = target {
                    let t = &mut self.targets[ti];
                    t.hp -= 1;
                    t.flash = 0.15;
                    if t.hp <= 0 {
                        t.respawn = Some(TARGET_RESPAWN);
                        self.score += 1;
                        let c = t.aabb.center;
                        self.explosions.push(Explosion {
                            pos: c,
                            age: 0.0,
                            duration: 0.8,
                            radius: 5.0,
                        });
                    }
                }
                self.shells.swap_remove(i);
            } else if s.life <= 0.0 || out_of_arena {
                self.shells.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn update_targets(&mut self, dt: f32) {
        let mut respawn = Vec::new();
        for (i, t) in self.targets.iter_mut().enumerate() {
            t.flash = (t.flash - dt).max(0.0);
            if let Some(r) = t.respawn.as_mut() {
                *r -= dt;
                if *r <= 0.0 {
                    respawn.push(i);
                }
            }
        }
        for i in respawn {
            let aabb = self.random_target_box();
            self.targets[i] = Target {
                aabb,
                hp: TARGET_HP,
                flash: 0.0,
                respawn: None,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shooting_a_target_destroys_it() {
        let mut w = World::new(7);
        // Put one target straight ahead and remove the rest.
        w.targets.truncate(1);
        w.targets[0].aabb = Aabb {
            center: Vector3::new(0.0, 1.5, 30.0),
            half: Vector3::new(2.0, 1.5, 2.0),
        };
        let fire = Controls {
            fire: true,
            ..Default::default()
        };
        let dt = 1.0 / 60.0;
        for _ in 0..(60 * 4) {
            w.update(&fire, dt);
        }
        assert!(w.score >= 1, "score was {}", w.score);
    }

    #[test]
    fn tank_cannot_drive_through_target() {
        let mut w = World::new(3);
        w.targets.truncate(1);
        w.targets[0].aabb = Aabb {
            center: Vector3::new(0.0, 1.5, 10.0),
            half: Vector3::new(2.0, 1.5, 2.0),
        };
        let go = Controls {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..300 {
            w.update(&go, 1.0 / 60.0);
        }
        assert!(w.tank.pos.z < 10.0 - 2.0 - tank::COLLISION_RADIUS + 0.01);
    }
}
