//! Small math helpers shared by the game logic.

use raylib::prelude::Vector3;
use std::f32::consts::{PI, TAU};

/// Unit vector for a yaw (around +Y) and pitch (up is positive).
/// Yaw 0 points along +Z.
pub fn dir_from_angles(yaw: f32, pitch: f32) -> Vector3 {
    Vector3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    )
}

/// Yaw of a horizontal direction, matching `dir_from_angles`.
pub fn yaw_of(v: Vector3) -> f32 {
    v.x.atan2(v.z)
}

/// Wraps an angle into (-PI, PI].
pub fn wrap_angle(a: f32) -> f32 {
    let mut a = (a + PI) % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a - PI
}

/// Moves `current` toward `target` by at most `max_step`, taking the short way round.
pub fn approach_angle(current: f32, target: f32, max_step: f32) -> f32 {
    let diff = wrap_angle(target - current);
    if diff.abs() <= max_step {
        target
    } else {
        current + max_step * diff.signum()
    }
}

/// Moves a value toward a target by at most `max_step`.
pub fn approach(current: f32, target: f32, max_step: f32) -> f32 {
    if (target - current).abs() <= max_step {
        target
    } else {
        current + max_step * (target - current).signum()
    }
}

/// Axis-aligned box given by its center and half extents.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub center: Vector3,
    pub half: Vector3,
}

impl Aabb {
    /// Distance along the ray to the first hit, if any (slab test).
    pub fn ray_hit(&self, origin: Vector3, dir: Vector3) -> Option<f32> {
        let mut t_min = 0.0f32;
        let mut t_max = f32::INFINITY;
        let o = [origin.x, origin.y, origin.z];
        let d = [dir.x, dir.y, dir.z];
        let c = [self.center.x, self.center.y, self.center.z];
        let h = [self.half.x, self.half.y, self.half.z];
        for i in 0..3 {
            if d[i].abs() < 1e-6 {
                if (o[i] - c[i]).abs() > h[i] {
                    return None;
                }
            } else {
                let t1 = (c[i] - h[i] - o[i]) / d[i];
                let t2 = (c[i] + h[i] - o[i]) / d[i];
                t_min = t_min.max(t1.min(t2));
                t_max = t_max.min(t1.max(t2));
                if t_min > t_max {
                    return None;
                }
            }
        }
        Some(t_min)
    }

    /// Pushes a circle (on the XZ plane) out of this box. Returns the corrected center.
    pub fn push_circle_out(&self, p: Vector3, radius: f32) -> Vector3 {
        let cx =
            p.x.clamp(self.center.x - self.half.x, self.center.x + self.half.x);
        let cz =
            p.z.clamp(self.center.z - self.half.z, self.center.z + self.half.z);
        let dx = p.x - cx;
        let dz = p.z - cz;
        let d2 = dx * dx + dz * dz;
        if d2 >= radius * radius {
            return p;
        }
        if d2 > 1e-8 {
            let d = d2.sqrt();
            let push = radius - d;
            Vector3::new(p.x + dx / d * push, p.y, p.z + dz / d * push)
        } else {
            // Center is inside the box: push out along the shallowest axis.
            let ox = self.half.x + radius - (p.x - self.center.x).abs();
            let oz = self.half.z + radius - (p.z - self.center.z).abs();
            if ox < oz {
                let s = if p.x >= self.center.x { 1.0 } else { -1.0 };
                Vector3::new(p.x + s * ox, p.y, p.z)
            } else {
                let s = if p.z >= self.center.z { 1.0 } else { -1.0 };
                Vector3::new(p.x, p.y, p.z + s * oz)
            }
        }
    }
}

/// Tiny deterministic random generator (xorshift), enough for spawning targets.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }

    pub fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_range() {
        assert!(
            (wrap_angle(3.0 * PI) - PI).abs() < 1e-4 || (wrap_angle(3.0 * PI) + PI).abs() < 1e-4
        );
        assert!((wrap_angle(0.5) - 0.5).abs() < 1e-6);
        assert!((wrap_angle(-TAU + 0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn approach_angle_takes_short_way() {
        // From 170 deg to -170 deg should go up through 180, not down through 0.
        let a = approach_angle(170f32.to_radians(), (-170f32).to_radians(), 0.05);
        assert!(a > 170f32.to_radians());
    }

    #[test]
    fn yaw_matches_direction() {
        for yaw in [-2.0f32, -0.5, 0.0, 1.0, 3.0] {
            assert!((wrap_angle(yaw_of(dir_from_angles(yaw, 0.0)) - yaw)).abs() < 1e-5);
        }
    }

    #[test]
    fn ray_hits_box_in_front() {
        let b = Aabb {
            center: Vector3::new(0.0, 0.0, 10.0),
            half: Vector3::new(1.0, 1.0, 1.0),
        };
        let t = b
            .ray_hit(Vector3::zero(), Vector3::new(0.0, 0.0, 1.0))
            .unwrap();
        assert!((t - 9.0).abs() < 1e-5);
        assert!(
            b.ray_hit(Vector3::zero(), Vector3::new(0.0, 0.0, -1.0))
                .is_none()
        );
    }

    #[test]
    fn circle_is_pushed_out() {
        let b = Aabb {
            center: Vector3::zero(),
            half: Vector3::new(1.0, 1.0, 1.0),
        };
        let p = b.push_circle_out(Vector3::new(1.5, 0.0, 0.0), 1.0);
        assert!((p.x - 2.0).abs() < 1e-5);
        let inside = b.push_circle_out(Vector3::new(0.2, 0.0, 0.0), 1.0);
        assert!((inside.x - 2.0).abs() < 1e-5);
    }
}
