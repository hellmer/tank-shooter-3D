//! Enemy tanks: they patrol, close in on the player, keep a fighting distance while circling,
//! steer around cover and fire their cannon when they have the player lined up.

use crate::enemy_tank_layout::{BARREL_LENGTH, BARREL_PIVOT, BOX_CENTER, BOX_HALF, TURRET_PIVOT};
use crate::math::{Aabb, Rng, approach, approach_angle, dir_from_angles, wrap_angle, yaw_of};
use raylib::prelude::Vector3;

pub const MAX_SPEED: f32 = 8.5;
pub const REVERSE_SPEED: f32 = 5.0;
pub const ACCEL: f32 = 2.2;
pub const HULL_TURN_SPEED: f32 = 1.3;
pub const TURRET_TURN_SPEED: f32 = 1.1;
pub const PITCH_SPEED: f32 = 0.6;
pub const COLLISION_RADIUS: f32 = 2.2;
pub const HP: i32 = 5;
pub const RELOAD: f32 = 3.6;
/// Closer than this, a tank hunts the player; farther, it patrols.
pub const SIGHT_RANGE: f32 = 85.0;
/// Range at which it starts shooting.
pub const FIRE_RANGE: f32 = 70.0;
/// It tries to stay between these distances from the player.
pub const NEAR: f32 = 22.0;
pub const FAR: f32 = 42.0;

/// What the tank is trying to do right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Wander between random points.
    Patrol,
    /// Drive toward the player.
    Close,
    /// Circle the player at fighting distance.
    Circle,
    /// Back away while facing the player.
    BackOff,
    /// Stuck against something: reverse and turn for a moment.
    Unstick,
}

pub struct EnemyTank {
    pub pos: Vector3,
    pub hull_yaw: f32,
    pub speed: f32,
    /// World yaw of the turret.
    pub turret_yaw: f32,
    pub barrel_pitch: f32,
    pub hp: i32,
    pub reload: f32,
    pub flash: f32,
    pub respawn: Option<f32>,
    pub track_offset: f32,
    pub mode: Mode,
    goal: Vector3,
    /// +1 or -1: which way it circles the player.
    orbit: f32,
    /// Seconds until it rethinks its goal.
    think: f32,
    /// Seconds spent trying to move without getting anywhere.
    stuck: f32,
    /// Seconds left in `Mode::Unstick`.
    unstick: f32,
}

/// Rotates a point by a yaw around +Y (same convention as `dir_from_angles`).
fn rotate_yaw(v: Vector3, yaw: f32) -> Vector3 {
    let (s, c) = yaw.sin_cos();
    Vector3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

fn inside_xz(b: &Aabb, p: Vector3, margin: f32) -> bool {
    (p.x - b.center.x).abs() < b.half.x + margin && (p.z - b.center.z).abs() < b.half.z + margin
}

impl EnemyTank {
    pub fn new(pos: Vector3, yaw: f32) -> EnemyTank {
        EnemyTank {
            pos,
            hull_yaw: yaw,
            speed: 0.0,
            turret_yaw: yaw,
            barrel_pitch: 0.0,
            hp: HP,
            // A grace period after appearing before the first shot.
            reload: 4.0,
            flash: 0.0,
            respawn: None,
            track_offset: 0.0,
            mode: Mode::Patrol,
            goal: pos,
            orbit: 1.0,
            think: 0.0,
            stuck: 0.0,
            unstick: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.respawn.is_none()
    }

    pub fn forward(&self) -> Vector3 {
        dir_from_angles(self.hull_yaw, 0.0)
    }

    /// Axis-aligned box around the turned hull, for shells and collisions.
    pub fn aabb(&self) -> Aabb {
        let (s, c) = self.hull_yaw.sin_cos();
        let (s, c) = (s.abs(), c.abs());
        // The corners of a turned box poke out further than its sides: shrink a little so
        // a diagonal tank is not hit by shells that clearly miss it.
        let hx = (BOX_HALF.x * c + BOX_HALF.z * s) * 0.9;
        let hz = (BOX_HALF.x * s + BOX_HALF.z * c) * 0.9;
        Aabb {
            center: self.pos + rotate_yaw(BOX_CENTER, self.hull_yaw),
            half: Vector3::new(hx, BOX_HALF.y, hz),
        }
    }

    pub fn turret_pivot_world(&self) -> Vector3 {
        self.pos + rotate_yaw(TURRET_PIVOT, self.hull_yaw)
    }

    pub fn barrel_pivot_world(&self) -> Vector3 {
        self.turret_pivot_world() + rotate_yaw(BARREL_PIVOT, self.turret_yaw)
    }

    pub fn barrel_dir(&self) -> Vector3 {
        dir_from_angles(self.turret_yaw, self.barrel_pitch)
    }

    pub fn muzzle(&self) -> Vector3 {
        self.barrel_pivot_world() + self.barrel_dir() * BARREL_LENGTH
    }

    /// Turret yaw relative to the hull, for drawing.
    pub fn turret_rel_yaw(&self) -> f32 {
        wrap_angle(self.turret_yaw - self.hull_yaw)
    }

    /// Picks a new goal for the hull. `player` is None when the player is dead.
    fn rethink(&mut self, player: Option<Vector3>, arena: f32, rng: &mut Rng) {
        self.think = rng.range(1.5, 3.5);
        if rng.next_f32() < 0.25 {
            self.orbit = -self.orbit;
        }
        let near_player = player.filter(|p| p.distance(self.pos) < SIGHT_RANGE);
        self.mode = match near_player {
            None => Mode::Patrol,
            Some(p) => {
                let d = p.distance(self.pos);
                if d > FAR {
                    Mode::Close
                } else if d < NEAR {
                    Mode::BackOff
                } else {
                    Mode::Circle
                }
            }
        };
        let lim = arena - 8.0;
        let clamp = |v: Vector3| Vector3::new(v.x.clamp(-lim, lim), 0.0, v.z.clamp(-lim, lim));
        self.goal = match (self.mode, near_player) {
            (Mode::Close, Some(p)) => {
                // Aim for a point beside the player rather than straight at it.
                let side = rotate_yaw(
                    Vector3::new(self.orbit * 12.0, 0.0, 0.0),
                    yaw_of(p - self.pos),
                );
                clamp(p + side)
            }
            (Mode::Circle, Some(p)) => {
                let away = self.pos - p;
                let d = (NEAR + FAR) * 0.5;
                let a = yaw_of(away) + self.orbit * 0.7;
                clamp(p + dir_from_angles(a, 0.0) * d)
            }
            (Mode::BackOff, Some(p)) => {
                let away = (self.pos - p).normalize();
                clamp(self.pos + away * 15.0)
            }
            _ => {
                // Patrol: somewhere new, not too far.
                let a = rng.range(0.0, std::f32::consts::TAU);
                let r = rng.range(15.0, 40.0);
                clamp(self.pos + dir_from_angles(a, 0.0) * r)
            }
        };
    }

    /// Steering: heading toward the goal, bent away from cover that is in the way.
    fn steer_heading(&self, want: f32, blockers: &[Aabb], arena: f32) -> f32 {
        let blocked = |yaw: f32, reach: f32| {
            let probe = self.pos + dir_from_angles(yaw, 0.0) * reach;
            probe.x.abs() > arena - 3.0
                || probe.z.abs() > arena - 3.0
                || blockers
                    .iter()
                    .any(|b| inside_xz(b, probe, COLLISION_RADIUS))
        };
        if !blocked(want, 4.0) && !blocked(want, 8.0) {
            return want;
        }
        // Try headings further and further to each side, preferring the side closer to `want`.
        for k in 1..=6 {
            let off = k as f32 * 0.35;
            for side in [1.0, -1.0] {
                let yaw = want + off * side;
                if !blocked(yaw, 4.0) && !blocked(yaw, 8.0) {
                    return yaw;
                }
            }
        }
        want + std::f32::consts::PI
    }

    /// Moves the hull for this frame. Collisions are resolved by the world afterwards.
    pub fn drive(
        &mut self,
        player: Option<Vector3>,
        blockers: &[Aabb],
        arena: f32,
        rng: &mut Rng,
        dt: f32,
    ) {
        self.think -= dt;
        if self.mode == Mode::Unstick {
            self.unstick -= dt;
            if self.unstick <= 0.0 {
                self.think = 0.0;
                self.mode = Mode::Patrol;
            }
        } else if self.think <= 0.0 || self.goal.distance(self.pos) < 4.0 {
            self.rethink(player, arena, rng);
        }

        let (want_yaw, throttle) = match self.mode {
            Mode::Unstick => (self.hull_yaw + self.orbit, -0.8),
            Mode::BackOff => {
                // Keep the nose (and armor) toward the player while reversing.
                let face = player.map_or(self.hull_yaw, |p| yaw_of(p - self.pos));
                let back = yaw_of(self.goal - self.pos) + std::f32::consts::PI;
                (
                    if wrap_angle(back - face).abs() < 0.8 {
                        back
                    } else {
                        face
                    },
                    -0.8,
                )
            }
            _ => {
                let raw = yaw_of(self.goal - self.pos);
                let heading = self.steer_heading(raw, blockers, arena);
                let diff = wrap_angle(heading - self.hull_yaw).abs();
                // Slow down for sharp turns and turn in place when the goal is behind.
                let t = if diff > 1.3 {
                    0.0
                } else {
                    (1.0 - diff / 1.3).max(0.25)
                };
                let t = if self.mode == Mode::Patrol {
                    t * 0.6
                } else {
                    t
                };
                (heading, t)
            }
        };

        self.hull_yaw = wrap_angle(approach_angle(
            self.hull_yaw,
            want_yaw,
            HULL_TURN_SPEED * dt,
        ));
        let target = if throttle >= 0.0 {
            throttle * MAX_SPEED
        } else {
            throttle * REVERSE_SPEED
        };
        self.speed += (target - self.speed) * (ACCEL * dt).min(1.0);
        self.pos += self.forward() * (self.speed * dt);
        self.track_offset += self.speed * dt;
    }

    /// Called after the world pushed the tank out of whatever it ran into.
    pub fn after_collisions(&mut self, moved: f32, dt: f32) {
        let trying = self.speed.abs() > 2.0 && self.mode != Mode::Unstick;
        if trying && moved < self.speed.abs() * dt * 0.3 {
            self.stuck += dt;
            self.speed *= 0.5;
        } else {
            self.stuck = (self.stuck - dt).max(0.0);
        }
        if self.stuck > 0.8 {
            self.stuck = 0.0;
            self.mode = Mode::Unstick;
            self.unstick = 1.2;
            self.orbit = -self.orbit;
        }
    }

    /// Turns the turret toward `aim` (already led) and says whether it is lined up to fire.
    pub fn aim(&mut self, aim: Option<Vector3>, dt: f32) -> bool {
        self.reload = (self.reload - dt).max(0.0);
        self.flash = (self.flash - dt).max(0.0);
        let Some(aim) = aim else {
            // Nobody to shoot: look where it is going.
            self.turret_yaw = wrap_angle(approach_angle(
                self.turret_yaw,
                self.hull_yaw,
                TURRET_TURN_SPEED * dt,
            ));
            self.barrel_pitch = approach(self.barrel_pitch, 0.0, PITCH_SPEED * dt);
            return false;
        };
        let want_yaw = yaw_of(aim - self.turret_pivot_world());
        let to = aim - self.barrel_pivot_world();
        let flat = (to.x * to.x + to.z * to.z).sqrt();
        let want_pitch = to.y.atan2(flat).clamp(-0.15, 0.35);
        self.turret_yaw = wrap_angle(approach_angle(
            self.turret_yaw,
            want_yaw,
            TURRET_TURN_SPEED * dt,
        ));
        self.barrel_pitch = approach(self.barrel_pitch, want_pitch, PITCH_SPEED * dt);
        let aligned = wrap_angle(want_yaw - self.turret_yaw).abs() < 0.05;
        aligned && self.reload <= 0.0 && flat < FIRE_RANGE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closes_in_on_a_far_player() {
        let mut t = EnemyTank::new(Vector3::new(0.0, 0.0, -70.0), 0.0);
        let player = Vector3::zero();
        let mut rng = Rng::new(1);
        let start = t.pos.distance(player);
        for _ in 0..(60 * 6) {
            t.drive(Some(player), &[], 90.0, &mut rng, 1.0 / 60.0);
        }
        assert!(t.pos.distance(player) < start - 20.0, "{:?}", t.pos);
    }

    #[test]
    fn steers_around_a_wall() {
        let wall = Aabb {
            center: Vector3::new(0.0, 1.0, -30.0),
            half: Vector3::new(6.0, 1.0, 1.0),
        };
        let mut t = EnemyTank::new(Vector3::new(0.0, 0.0, -60.0), 0.0);
        let mut rng = Rng::new(2);
        for _ in 0..(60 * 10) {
            let before = t.pos;
            t.drive(Some(Vector3::zero()), &[wall], 90.0, &mut rng, 1.0 / 60.0);
            t.pos = wall.push_circle_out(t.pos, COLLISION_RADIUS);
            t.after_collisions(t.pos.distance(before), 1.0 / 60.0);
        }
        assert!(t.pos.z > -25.0, "stuck behind the wall at {:?}", t.pos);
    }

    #[test]
    fn turret_lines_up_and_reports_ready() {
        let mut t = EnemyTank::new(Vector3::zero(), 0.0);
        t.reload = 0.0;
        let aim = Vector3::new(30.0, 1.0, 0.0);
        let mut ready = false;
        for _ in 0..(60 * 3) {
            ready |= t.aim(Some(aim), 1.0 / 60.0);
        }
        assert!(ready);
        assert!(t.barrel_dir().x > 0.95);
    }
}
