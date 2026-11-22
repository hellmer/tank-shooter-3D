//! The player's tank: hull movement, turret and barrel aiming.

use crate::input::Controls;
use crate::math::{approach, approach_angle, dir_from_angles, wrap_angle, yaw_of};
use raylib::prelude::Vector3;

pub const MAX_SPEED: f32 = 12.0;
pub const REVERSE_SPEED: f32 = 6.0;
pub const ACCEL: f32 = 3.0;
pub const HULL_TURN_SPEED: f32 = 1.6;
pub const TURRET_TURN_SPEED: f32 = 1.8;
pub const BARREL_PITCH_SPEED: f32 = 1.0;
pub const BARREL_MIN_PITCH: f32 = -0.12;
pub const BARREL_MAX_PITCH: f32 = 0.35;
pub const RELOAD_TIME: f32 = 0.8;
pub const COLLISION_RADIUS: f32 = 2.4;

// Pivots and barrel length come from the 3dixel model, so the game and the art agree.
pub use crate::model_layout::{BARREL_LENGTH, BARREL_PIVOT, TURRET_PIVOT};

pub struct Tank {
    pub pos: Vector3,
    pub hull_yaw: f32,
    pub speed: f32,
    /// Turret yaw relative to the hull.
    pub turret_yaw: f32,
    pub barrel_pitch: f32,
    pub reload: f32,
    /// Distance travelled, used to animate the tracks.
    pub track_offset: f32,
}

/// Rotates a point by a yaw around +Y (same convention as `dir_from_angles`).
fn rotate_yaw(v: Vector3, yaw: f32) -> Vector3 {
    let (s, c) = yaw.sin_cos();
    Vector3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

impl Tank {
    pub fn new(pos: Vector3) -> Tank {
        Tank {
            pos,
            hull_yaw: 0.0,
            speed: 0.0,
            turret_yaw: 0.0,
            barrel_pitch: 0.0,
            reload: 0.0,
            track_offset: 0.0,
        }
    }

    pub fn forward(&self) -> Vector3 {
        dir_from_angles(self.hull_yaw, 0.0)
    }

    pub fn turret_world_yaw(&self) -> f32 {
        self.hull_yaw + self.turret_yaw
    }

    pub fn turret_pivot_world(&self) -> Vector3 {
        self.pos + rotate_yaw(TURRET_PIVOT, self.hull_yaw)
    }

    pub fn barrel_pivot_world(&self) -> Vector3 {
        self.turret_pivot_world() + rotate_yaw(BARREL_PIVOT, self.turret_world_yaw())
    }

    pub fn barrel_dir(&self) -> Vector3 {
        dir_from_angles(self.turret_world_yaw(), self.barrel_pitch)
    }

    pub fn muzzle(&self) -> Vector3 {
        self.barrel_pivot_world() + self.barrel_dir() * BARREL_LENGTH
    }

    /// Drives the hull from the controls.
    pub fn drive(&mut self, c: &Controls, dt: f32) {
        let target = if c.throttle >= 0.0 {
            c.throttle * MAX_SPEED
        } else {
            c.throttle * REVERSE_SPEED
        };
        self.speed += (target - self.speed) * (ACCEL * dt).min(1.0);
        self.hull_yaw = wrap_angle(self.hull_yaw + c.steer * HULL_TURN_SPEED * dt);
        self.pos += self.forward() * (self.speed * dt);
        self.track_offset += self.speed * dt;
        self.reload = (self.reload - dt).max(0.0);
    }

    /// Turns the turret and barrel toward a point in the world, at their limited speeds.
    pub fn aim_at(&mut self, point: Vector3, dt: f32) {
        let pivot = self.barrel_pivot_world();
        let to = point - pivot;
        let flat = (to.x * to.x + to.z * to.z).sqrt();
        if flat < 0.5 {
            return;
        }
        // Yaw is measured from the turret pivot so a near target does not make it spin.
        let from_turret = point - self.turret_pivot_world();
        let want_world_yaw = yaw_of(from_turret);
        let want_rel = wrap_angle(want_world_yaw - self.hull_yaw);
        self.turret_yaw = wrap_angle(approach_angle(
            self.turret_yaw,
            want_rel,
            TURRET_TURN_SPEED * dt,
        ));
        let want_pitch = to.y.atan2(flat).clamp(BARREL_MIN_PITCH, BARREL_MAX_PITCH);
        self.barrel_pitch = approach(self.barrel_pitch, want_pitch, BARREL_PITCH_SPEED * dt);
    }

    pub fn can_fire(&self) -> bool {
        self.reload <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drives_forward_along_hull_yaw() {
        let mut t = Tank::new(Vector3::zero());
        let c = Controls {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            t.drive(&c, 1.0 / 60.0);
        }
        assert!(t.pos.z > 5.0);
        assert!(t.pos.x.abs() < 1e-3);
    }

    #[test]
    fn turret_turns_toward_point_over_time() {
        let mut t = Tank::new(Vector3::zero());
        let target = Vector3::new(30.0, 0.0, 0.0); // to the left in this convention (+X)
        t.aim_at(target, 0.1);
        assert!(t.turret_yaw > 0.0 && t.turret_yaw < 0.2);
        for _ in 0..100 {
            t.aim_at(target, 0.1);
        }
        let d = t.barrel_dir();
        assert!(d.x > 0.95);
    }
}
