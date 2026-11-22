//! Game state: the tank, crates, enemy turrets, shells and effects, and how they interact.

use crate::enemy_tank::{self, EnemyTank};
use crate::input::Controls;
use crate::math::{Aabb, Rng, approach, approach_angle, dir_from_angles, wrap_angle, yaw_of};
use crate::props_layout;
use crate::tank::{self, Tank};
use crate::turret_layout;
use raylib::prelude::Vector3;

pub const ARENA_HALF: f32 = 90.0;
pub const SHELL_SPEED: f32 = 80.0;
pub const SHELL_LIFE: f32 = 3.0;
pub const MAX_AIM_RANGE: f32 = 250.0;
pub const TARGET_COUNT: usize = 14;
pub const ROCK_COUNT: usize = 9;
pub const BARRIER_COUNT: usize = 10;
pub const TARGET_HP: i32 = 3;
pub const TARGET_RESPAWN: f32 = 4.0;

pub const PLAYER_MAX_HP: f32 = 100.0;
pub const PLAYER_HIT_HALF: Vector3 = Vector3 {
    x: 2.1,
    y: 1.3,
    z: 2.1,
};

pub const ENEMY_COUNT: usize = 5;
pub const ENEMY_HP: i32 = 4;
pub const ENEMY_RANGE: f32 = 65.0;
pub const ENEMY_TURN_SPEED: f32 = 0.9;
pub const ENEMY_PITCH_SPEED: f32 = 0.6;
pub const ENEMY_RELOAD: f32 = 2.4;
pub const ENEMY_SHELL_SPEED: f32 = 55.0;
pub const ENEMY_DAMAGE: f32 = 12.0;
pub const ENEMY_SPREAD: f32 = 0.025;
pub const ENEMY_RESPAWN: f32 = 8.0;
pub const ENEMY_HALF_WIDTH: f32 = 1.4;

pub const ENEMY_TANK_COUNT: usize = 3;
pub const ENEMY_TANK_DAMAGE: f32 = 12.0;
pub const ENEMY_TANK_SHELL_SPEED: f32 = 60.0;
pub const ENEMY_TANK_SPREAD: f32 = 0.04;
pub const ENEMY_TANK_RESPAWN: f32 = 10.0;

const GRAVITY: f32 = 18.0;

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

    /// A quarter turn picked from the position, so crates do not all face the same way.
    pub fn yaw(&self) -> f32 {
        let k = (self.aabb.center.x * 7.0 + self.aabb.center.z * 13.0).abs() as i32 % 4;
        k as f32 * std::f32::consts::FRAC_PI_2
    }
}

/// A crate's hit box when its base sits at `base`.
pub fn crate_box(base: Vector3) -> Aabb {
    Aabb {
        center: base + props_layout::CRATE_CENTER,
        half: props_layout::CRATE_HALF,
    }
}

/// Pushes `p` away from `other` (on the ground plane) until they are `dist` apart.
fn push_apart(p: Vector3, other: Vector3, dist: f32) -> Vector3 {
    let d = Vector3::new(p.x - other.x, 0.0, p.z - other.z);
    let len = d.length();
    if len >= dist {
        return p;
    }
    if len < 1e-4 {
        return p + Vector3::new(dist, 0.0, 0.0);
    }
    p + d * ((dist - len) / len)
}

fn overlaps(a: &Aabb, b: &Aabb) -> bool {
    (a.center.x - b.center.x).abs() < a.half.x + b.half.x
        && (a.center.z - b.center.z).abs() < a.half.z + b.half.z
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ObstacleKind {
    Rock,
    Barrier,
}

/// Fixed cover that shells cannot destroy.
pub struct Obstacle {
    pub kind: ObstacleKind,
    /// Base on the ground.
    pub pos: Vector3,
    /// Quarter turns (0..4), so the hit box stays axis-aligned.
    pub quarter: u8,
}

impl Obstacle {
    pub fn yaw(&self) -> f32 {
        self.quarter as f32 * std::f32::consts::FRAC_PI_2
    }

    pub fn aabb(&self) -> Aabb {
        let (c, h) = match self.kind {
            ObstacleKind::Rock => (props_layout::ROCK_CENTER, props_layout::ROCK_HALF),
            ObstacleKind::Barrier => (props_layout::BARRIER_CENTER, props_layout::BARRIER_HALF),
        };
        // Same rotation as raylib's yaw around +Y: (x, z) -> (x cos + z sin, -x sin + z cos).
        let (c, h) = match self.quarter % 4 {
            0 => (c, h),
            1 => (Vector3::new(c.z, c.y, -c.x), Vector3::new(h.z, h.y, h.x)),
            2 => (Vector3::new(-c.x, c.y, -c.z), h),
            _ => (Vector3::new(-c.z, c.y, c.x), Vector3::new(h.z, h.y, h.x)),
        };
        Aabb {
            center: self.pos + c,
            half: h,
        }
    }
}

/// A fixed enemy turret that turns toward the player and fires when it has a clear shot.
pub struct Enemy {
    /// Base of the turret, on the ground.
    pub pos: Vector3,
    /// World yaw of the head.
    pub yaw: f32,
    pub pitch: f32,
    pub hp: i32,
    pub reload: f32,
    pub flash: f32,
    pub respawn: Option<f32>,
    /// Which of the two barrels fires next.
    pub next_barrel: bool,
}

impl Enemy {
    pub fn alive(&self) -> bool {
        self.respawn.is_none()
    }

    pub fn aabb(&self) -> Aabb {
        let h = turret_layout::HEIGHT / 2.0;
        Aabb {
            center: self.pos + Vector3::new(0.0, h, 0.0),
            half: Vector3::new(ENEMY_HALF_WIDTH, h, ENEMY_HALF_WIDTH),
        }
    }

    fn gun_pivot_world(&self) -> Vector3 {
        let p = turret_layout::GUN_PIVOT;
        let (s, c) = self.yaw.sin_cos();
        self.pos
            + turret_layout::HEAD_PIVOT
            + Vector3::new(p.x * c + p.z * s, p.y, -p.x * s + p.z * c)
    }

    pub fn gun_dir(&self) -> Vector3 {
        dir_from_angles(self.yaw, self.pitch)
    }

    pub fn muzzle(&self, right_barrel: bool) -> Vector3 {
        let side = if right_barrel { -1.0 } else { 1.0 };
        let (s, c) = self.yaw.sin_cos();
        let across = Vector3::new(c, 0.0, -s) * (turret_layout::MUZZLE_X * side);
        self.gun_pivot_world() + self.gun_dir() * turret_layout::GUN_LENGTH + across
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Owner {
    Player,
    Enemy,
}

pub struct Shell {
    pub pos: Vector3,
    pub vel: Vector3,
    pub life: f32,
    pub owner: Owner,
    /// Armor taken from the player on a hit (enemy shells only).
    pub damage: f32,
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

/// A chunk thrown out by a destroyed turret, crate or tank.
pub struct Debris {
    pub pos: Vector3,
    pub vel: Vector3,
    pub spin: f32,
    pub angle: f32,
    pub size: f32,
    pub life: f32,
    /// 0 = steel, 1 = crate wood, 2 = glowing ember.
    pub kind: u8,
}

/// A sound the game wants played this frame, and where it happens.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx {
    PlayerCannon,
    EnemyCannon,
    TurretShot,
    /// A shell bursting on the ground, cover or a crate.
    Impact,
    /// Something big blowing up: a turret, a tank.
    Explosion,
    /// The player's armor taking a hit.
    HitArmor,
    /// The player's shell hitting an enemy that survives it.
    HitEnemy,
    CrateBreak,
    /// The player's gun is loaded again.
    Reload,
}

#[derive(Clone, Copy, Debug)]
pub struct SoundEvent {
    pub sfx: Sfx,
    pub pos: Vector3,
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
    pub hp: f32,
    pub camera: OrbitCamera,
    pub targets: Vec<Target>,
    pub obstacles: Vec<Obstacle>,
    pub enemies: Vec<Enemy>,
    pub enemy_tanks: Vec<EnemyTank>,
    pub shells: Vec<Shell>,
    pub explosions: Vec<Explosion>,
    pub debris: Vec<Debris>,
    pub crates_destroyed: u32,
    pub enemies_destroyed: u32,
    pub aim_point: Vector3,
    /// Seconds of red flash left after the player is hit.
    pub hurt_flash: f32,
    /// Camera shake strength; decays every frame.
    pub shake: f32,
    pub time: f32,
    /// Sounds started during the last `update`, for the audio code to play.
    pub sounds: Vec<SoundEvent>,
    seed: u64,
    rng: Rng,
}

impl World {
    pub fn new(seed: u64) -> World {
        let mut w = World {
            tank: Tank::new(Vector3::zero()),
            hp: PLAYER_MAX_HP,
            camera: OrbitCamera {
                yaw: 0.0,
                pitch: -0.1,
                distance: 13.0,
            },
            targets: Vec::new(),
            obstacles: Vec::new(),
            enemies: Vec::new(),
            enemy_tanks: Vec::new(),
            shells: Vec::new(),
            explosions: Vec::new(),
            debris: Vec::new(),
            crates_destroyed: 0,
            enemies_destroyed: 0,
            aim_point: Vector3::new(0.0, 0.0, 50.0),
            hurt_flash: 0.0,
            shake: 0.0,
            time: 0.0,
            sounds: Vec::new(),
            seed,
            rng: Rng::new(seed),
        };
        for k in 0..(ROCK_COUNT + BARRIER_COUNT) {
            let kind = if k < ROCK_COUNT {
                ObstacleKind::Rock
            } else {
                ObstacleKind::Barrier
            };
            let pos = w.random_spot(14.0, 80.0, 9.0);
            let quarter = (w.rng.next_f32() * 4.0) as u8 % 4;
            w.obstacles.push(Obstacle { kind, pos, quarter });
        }
        for _ in 0..TARGET_COUNT {
            let aabb = w.random_target_box();
            w.targets.push(Target {
                aabb,
                hp: TARGET_HP,
                flash: 0.0,
                respawn: None,
            });
        }
        for _ in 0..ENEMY_COUNT {
            let pos = w.random_enemy_spot();
            let yaw = w.rng.range(-3.1, 3.1);
            w.enemies.push(Enemy {
                pos,
                yaw,
                pitch: 0.0,
                hp: ENEMY_HP,
                reload: 1.5,
                flash: 0.0,
                respawn: None,
                next_barrel: false,
            });
        }
        for _ in 0..ENEMY_TANK_COUNT {
            let pos = w.random_enemy_spot();
            let yaw = w.rng.range(-3.1, 3.1);
            w.enemy_tanks.push(EnemyTank::new(pos, yaw));
        }
        w
    }

    pub fn player_alive(&self) -> bool {
        self.hp > 0.0
    }

    /// How much of a fight the player is in, 0 (quiet) to 1, for the music: a hunting tank or a
    /// turret in range, or a recent hit, counts as a fight.
    pub fn danger(&self) -> f32 {
        if !self.player_alive() {
            return 0.0;
        }
        let p = self.tank.pos;
        let tanks = self
            .enemy_tanks
            .iter()
            .filter(|t| t.alive() && t.pos.distance(p) < enemy_tank::SIGHT_RANGE)
            .count();
        let turrets = self
            .enemies
            .iter()
            .filter(|e| e.alive() && e.pos.distance(p) < ENEMY_RANGE)
            .count();
        let hurt = if self.hurt_flash > 0.0 { 1.0 } else { 0.0 };
        (tanks as f32 * 0.6 + turrets as f32 * 0.35 + hurt).min(1.0)
    }

    fn spot_is_clear(&self, center: Vector3, clearance: f32) -> bool {
        center.distance(self.tank.pos) > 25.0
            && self
                .targets
                .iter()
                .all(|t| t.aabb.center.distance(center) > clearance)
            && self
                .enemies
                .iter()
                .all(|e| e.pos.distance(center) > clearance + 4.0)
            && self
                .enemy_tanks
                .iter()
                .all(|t| t.pos.distance(center) > clearance + 4.0)
            && self
                .obstacles
                .iter()
                .all(|o| o.aabb().center.distance(center) > clearance)
    }

    fn random_spot(&mut self, min_dist: f32, max_dist: f32, clearance: f32) -> Vector3 {
        loop {
            let angle = self.rng.range(0.0, std::f32::consts::TAU);
            let dist = self.rng.range(min_dist, max_dist);
            let p = Vector3::new(angle.sin() * dist, 0.0, angle.cos() * dist);
            let inside = p.x.abs() < ARENA_HALF - 6.0 && p.z.abs() < ARENA_HALF - 6.0;
            if inside && p.distance(self.tank.pos) > 10.0 && self.spot_is_clear(p, clearance) {
                return p;
            }
        }
    }

    fn random_target_box(&mut self) -> Aabb {
        // Half the crates are stacked against cover, the rest stand alone.
        if !self.obstacles.is_empty() && self.rng.next_f32() < 0.5 {
            for _ in 0..12 {
                let k = (self.rng.next_f32() * self.obstacles.len() as f32) as usize
                    % self.obstacles.len();
                let o = self.obstacles[k].aabb();
                let side = self.rng.range(0.0, std::f32::consts::TAU);
                let reach = o.half.x.max(o.half.z) + props_layout::CRATE_HALF.x + 0.3;
                let base = Vector3::new(
                    o.center.x + side.sin() * reach,
                    0.0,
                    o.center.z + side.cos() * reach,
                );
                let b = crate_box(base);
                let free = self
                    .targets
                    .iter()
                    .all(|t| t.aabb.center.distance(b.center) > 3.0)
                    && self.obstacles.iter().all(|ob| !overlaps(&ob.aabb(), &b))
                    && self.enemies.iter().all(|en| !overlaps(&en.aabb(), &b))
                    && base.distance(self.tank.pos) > 10.0
                    && base.x.abs() < ARENA_HALF - 3.0
                    && base.z.abs() < ARENA_HALF - 3.0;
                if free {
                    return b;
                }
            }
        }
        let base = self.random_spot(18.0, 75.0, 5.0);
        crate_box(base)
    }

    fn random_enemy_spot(&mut self) -> Vector3 {
        loop {
            let angle = self.rng.range(0.0, std::f32::consts::TAU);
            let dist = self.rng.range(35.0, 80.0);
            let p = Vector3::new(angle.sin() * dist, 0.0, angle.cos() * dist);
            let inside = p.x.abs() < ARENA_HALF - 6.0 && p.z.abs() < ARENA_HALF - 6.0;
            if inside && self.spot_is_clear(p, 8.0) {
                return p;
            }
        }
    }

    /// Where a ray first hits the ground, a crate or an enemy.
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
        for e in self.enemies.iter().filter(|e| e.alive()) {
            if let Some(d) = e.aabb().ray_hit(origin, dir) {
                best = best.min(d);
            }
        }
        for t in self.enemy_tanks.iter().filter(|t| t.alive()) {
            if let Some(d) = t.aabb().ray_hit(origin, dir) {
                best = best.min(d);
            }
        }
        for o in &self.obstacles {
            if let Some(d) = o.aabb().ray_hit(origin, dir) {
                best = best.min(d);
            }
        }
        origin + dir * best
    }

    /// Camera position and look target for this frame, with shake applied.
    pub fn camera_pose(&self) -> (Vector3, Vector3) {
        let focus = self.tank.pos + Vector3::new(0.0, 4.2, 0.0);
        let dir = self.camera.dir();
        let mut eye = focus - dir * self.camera.distance;
        eye.y = eye.y.max(0.6);
        let s = self.shake * self.shake;
        let jitter = Vector3::new(
            (self.time * 61.0).sin(),
            (self.time * 47.0).sin(),
            (self.time * 53.0).cos(),
        ) * (0.35 * s);
        (eye + jitter, focus + dir * 10.0 + jitter)
    }

    fn player_aabb(&self) -> Aabb {
        Aabb {
            center: self.tank.pos + Vector3::new(0.0, PLAYER_HIT_HALF.y, 0.0),
            half: PLAYER_HIT_HALF,
        }
    }

    fn burst(&mut self, at: Vector3, count: usize, speed: f32, kind: u8) {
        for i in 0..count {
            let yaw = self.rng.range(0.0, std::f32::consts::TAU);
            let up = self.rng.range(0.35, 1.0);
            let v = Vector3::new(
                yaw.sin() * (1.0 - up * 0.5),
                up,
                yaw.cos() * (1.0 - up * 0.5),
            ) * (speed * self.rng.range(0.5, 1.0));
            let ember = kind == 0 && i % 4 == 0;
            self.debris.push(Debris {
                pos: at,
                vel: v,
                spin: self.rng.range(-8.0, 8.0),
                angle: 0.0,
                size: self.rng.range(0.15, 0.45),
                life: self.rng.range(1.2, 2.4),
                kind: if ember { 2 } else { kind },
            });
        }
    }

    fn sound(&mut self, sfx: Sfx, pos: Vector3) {
        self.sounds.push(SoundEvent { sfx, pos });
    }

    pub fn update(&mut self, c: &Controls, dt: f32) {
        self.sounds.clear();
        self.time += dt;
        if c.restart && !self.player_alive() {
            *self = World::new(self.seed.wrapping_add(1));
            return;
        }

        // Camera.
        self.camera.yaw += c.look_yaw;
        self.camera.pitch = (self.camera.pitch + c.look_pitch)
            .clamp(OrbitCamera::MIN_PITCH, OrbitCamera::MAX_PITCH);

        // Tank movement and collisions. A destroyed tank just rolls to a stop.
        let drive = if self.player_alive() {
            *c
        } else {
            Controls::default()
        };
        let was_loading = self.tank.reload > 0.0;
        self.tank.drive(&drive, dt);
        if was_loading && self.tank.can_fire() && self.player_alive() {
            let at = self.tank.turret_pivot_world();
            self.sound(Sfx::Reload, at);
        }
        let blockers: Vec<Aabb> = self
            .targets
            .iter()
            .filter(|t| t.alive())
            .map(|t| t.aabb)
            .chain(self.enemies.iter().filter(|e| e.alive()).map(|e| e.aabb()))
            .chain(self.obstacles.iter().map(|o| o.aabb()))
            .collect();
        for t in self.enemy_tanks.iter().filter(|t| t.alive()) {
            self.tank.pos = push_apart(
                self.tank.pos,
                t.pos,
                tank::COLLISION_RADIUS + enemy_tank::COLLISION_RADIUS,
            );
        }
        for b in blockers {
            let pushed = b.push_circle_out(self.tank.pos, tank::COLLISION_RADIUS);
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
        if self.player_alive() {
            self.tank.aim_at(self.aim_point, dt);
        }

        // Fire.
        if self.player_alive() && c.fire && self.tank.can_fire() {
            self.tank.reload = tank::RELOAD_TIME;
            let muzzle = self.tank.muzzle();
            self.sound(Sfx::PlayerCannon, muzzle);
            self.shells.push(Shell {
                pos: muzzle,
                vel: self.tank.barrel_dir() * SHELL_SPEED,
                life: SHELL_LIFE,
                owner: Owner::Player,
                damage: 0.0,
            });
            self.explosions.push(Explosion {
                pos: muzzle,
                age: 0.0,
                duration: 0.12,
                radius: 0.7,
            });
            self.tank.speed -= 1.5;
            self.shake = self.shake.max(0.35);
        }

        self.update_enemies(dt);
        self.update_enemy_tanks(dt);
        self.update_shells(dt);
        self.update_targets(dt);
        self.update_effects(dt);
    }

    fn update_enemies(&mut self, dt: f32) {
        let player = self.tank.pos + Vector3::new(0.0, 1.2, 0.0);
        let player_vel = self.tank.forward() * self.tank.speed;
        let alive = self.player_alive();
        let mut shots = Vec::new();
        for i in 0..self.enemies.len() {
            let e = &mut self.enemies[i];
            e.flash = (e.flash - dt).max(0.0);
            if let Some(r) = e.respawn.as_mut() {
                *r -= dt;
                if *r <= 0.0 {
                    let pos = self.random_enemy_spot();
                    let e = &mut self.enemies[i];
                    *e = Enemy {
                        pos,
                        yaw: e.yaw,
                        pitch: 0.0,
                        hp: ENEMY_HP,
                        reload: 2.0,
                        flash: 0.0,
                        respawn: None,
                        next_barrel: false,
                    };
                }
                continue;
            }
            e.reload = (e.reload - dt).max(0.0);
            let dist = e.pos.distance(self.tank.pos);
            if !alive || dist > ENEMY_RANGE {
                // Slow idle sweep.
                e.yaw = wrap_angle(e.yaw + 0.25 * dt);
                e.pitch = approach(e.pitch, 0.0, ENEMY_PITCH_SPEED * dt);
                continue;
            }
            // Lead the target a little.
            let t_flight = dist / ENEMY_SHELL_SPEED;
            let aim = player + player_vel * (t_flight * 0.8);
            let pivot = e.muzzle(false) - e.gun_dir() * turret_layout::GUN_LENGTH;
            let to = aim - pivot;
            let flat = (to.x * to.x + to.z * to.z).sqrt();
            let want_yaw = yaw_of(aim - e.pos);
            let want_pitch = to.y.atan2(flat).clamp(-0.3, 0.5);
            e.yaw = wrap_angle(approach_angle(e.yaw, want_yaw, ENEMY_TURN_SPEED * dt));
            e.pitch = approach(e.pitch, want_pitch, ENEMY_PITCH_SPEED * dt);
            let aligned = wrap_angle(want_yaw - e.yaw).abs() < 0.06;
            if aligned && e.reload <= 0.0 {
                let right = e.next_barrel;
                e.next_barrel = !e.next_barrel;
                e.reload = ENEMY_RELOAD;
                let muzzle = e.muzzle(right);
                shots.push((i, muzzle, e.yaw, e.pitch));
            }
        }
        for (i, muzzle, yaw, pitch) in shots {
            if self.shot_blocked(muzzle, dir_from_angles(yaw, pitch)) {
                self.enemies[i].reload = 0.6;
                continue;
            }
            self.enemy_fire(
                muzzle,
                yaw,
                pitch,
                ENEMY_SPREAD,
                ENEMY_SHELL_SPEED,
                ENEMY_DAMAGE,
                0.5,
            );
            self.sound(Sfx::TurretShot, muzzle);
        }
    }

    /// True when cover or a crate stands between `muzzle` and the player along `dir`,
    /// so enemies hold fire instead of wasting shells on it.
    fn shot_blocked(&self, muzzle: Vector3, dir: Vector3) -> bool {
        let to_player = (self.tank.pos - muzzle).length();
        self.targets
            .iter()
            .filter(|t| t.alive())
            .any(|t| t.aabb.ray_hit(muzzle, dir).is_some_and(|d| d < to_player))
            || self
                .obstacles
                .iter()
                .any(|o| o.aabb().ray_hit(muzzle, dir).is_some_and(|d| d < to_player))
    }

    #[allow(clippy::too_many_arguments)]
    fn enemy_fire(
        &mut self,
        muzzle: Vector3,
        yaw: f32,
        pitch: f32,
        spread: f32,
        speed: f32,
        damage: f32,
        flash: f32,
    ) {
        let dy = self.rng.range(-spread, spread);
        let dp = self.rng.range(-spread, spread) * 0.5;
        let vel = dir_from_angles(yaw + dy, pitch + dp) * speed;
        self.shells.push(Shell {
            pos: muzzle,
            vel,
            life: SHELL_LIFE,
            owner: Owner::Enemy,
            damage,
        });
        self.explosions.push(Explosion {
            pos: muzzle,
            age: 0.0,
            duration: 0.1,
            radius: flash,
        });
    }

    fn update_enemy_tanks(&mut self, dt: f32) {
        let alive = self.player_alive();
        let player = self.tank.pos + Vector3::new(0.0, 1.2, 0.0);
        let player_vel = self.tank.forward() * self.tank.speed;
        // Fixed things every enemy tank must steer around and cannot drive through.
        let fixed: Vec<Aabb> = self
            .targets
            .iter()
            .filter(|t| t.alive())
            .map(|t| t.aabb)
            .chain(self.enemies.iter().filter(|e| e.alive()).map(|e| e.aabb()))
            .chain(self.obstacles.iter().map(|o| o.aabb()))
            .collect();
        let mut shots = Vec::new();
        for i in 0..self.enemy_tanks.len() {
            if let Some(r) = self.enemy_tanks[i].respawn.as_mut() {
                *r -= dt;
                if *r <= 0.0 {
                    let pos = self.random_enemy_spot();
                    let yaw = yaw_of(self.tank.pos - pos);
                    self.enemy_tanks[i] = EnemyTank::new(pos, yaw);
                }
                continue;
            }
            let t = &mut self.enemy_tanks[i];
            let before = t.pos;
            let seen = alive.then_some(self.tank.pos);
            t.drive(seen, &fixed, ARENA_HALF, &mut self.rng, dt);
            let others: Vec<Vector3> = self
                .enemy_tanks
                .iter()
                .enumerate()
                .filter(|(j, o)| *j != i && o.alive())
                .map(|(_, o)| o.pos)
                .collect();
            let t = &mut self.enemy_tanks[i];
            for b in &fixed {
                t.pos = b.push_circle_out(t.pos, enemy_tank::COLLISION_RADIUS);
            }
            for o in others {
                t.pos = push_apart(t.pos, o, enemy_tank::COLLISION_RADIUS * 2.0);
            }
            if alive {
                t.pos = push_apart(
                    t.pos,
                    self.tank.pos,
                    tank::COLLISION_RADIUS + enemy_tank::COLLISION_RADIUS,
                );
            }
            let lim = ARENA_HALF - enemy_tank::COLLISION_RADIUS;
            t.pos.x = t.pos.x.clamp(-lim, lim);
            t.pos.z = t.pos.z.clamp(-lim, lim);
            let moved = (t.pos - before).length();
            t.after_collisions(moved, dt);

            // Turret: lead the player a little, like the turrets do.
            let dist = t.pos.distance(self.tank.pos);
            let aim = (alive && dist < enemy_tank::SIGHT_RANGE).then(|| {
                let flight = dist / ENEMY_TANK_SHELL_SPEED;
                player + player_vel * (flight * 0.7)
            });
            if t.aim(aim, dt) {
                t.reload = enemy_tank::RELOAD;
                shots.push((i, t.muzzle(), t.turret_yaw, t.barrel_pitch));
            }
        }
        for (i, muzzle, yaw, pitch) in shots {
            if self.shot_blocked(muzzle, dir_from_angles(yaw, pitch)) {
                // Look for a clear line instead of firing into cover.
                self.enemy_tanks[i].reload = 0.8;
                continue;
            }
            self.enemy_tanks[i].speed -= 1.0;
            self.enemy_fire(
                muzzle,
                yaw,
                pitch,
                ENEMY_TANK_SPREAD,
                ENEMY_TANK_SHELL_SPEED,
                ENEMY_TANK_DAMAGE,
                0.7,
            );
            self.sound(Sfx::EnemyCannon, muzzle);
        }
    }

    fn update_shells(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.shells.len() {
            let s = &mut self.shells[i];
            let step = s.vel * dt;
            let start = s.pos;
            let owner = s.owner;
            let damage = s.damage;
            s.pos += step;
            s.life -= dt;
            let len = step.length();
            let dir = if len > 0.0 { step * (1.0 / len) } else { step };
            let end = s.pos;

            #[derive(Clone, Copy)]
            enum Hit {
                Ground,
                Crate(usize),
                Enemy(usize),
                EnemyTank(usize),
                Player,
            }
            // Swept hit test so fast shells do not pass through thin boxes.
            let mut hit: Option<(f32, Hit)> = None;
            let mut consider = |d: f32, h: Hit| {
                if d <= len && hit.is_none_or(|(best, _)| d < best) {
                    hit = Some((d, h));
                }
            };
            if end.y <= 0.0 && dir.y < 0.0 {
                consider(-start.y / dir.y, Hit::Ground);
            }
            for (ti, t) in self.targets.iter().enumerate().filter(|(_, t)| t.alive()) {
                if let Some(d) = t.aabb.ray_hit(start, dir) {
                    consider(d, Hit::Crate(ti));
                }
            }
            for o in &self.obstacles {
                if let Some(d) = o.aabb().ray_hit(start, dir) {
                    consider(d, Hit::Ground);
                }
            }
            if owner == Owner::Player {
                for (ei, e) in self.enemies.iter().enumerate().filter(|(_, e)| e.alive()) {
                    if let Some(d) = e.aabb().ray_hit(start, dir) {
                        consider(d, Hit::Enemy(ei));
                    }
                }
                for (ti, t) in self
                    .enemy_tanks
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.alive())
                {
                    if let Some(d) = t.aabb().ray_hit(start, dir) {
                        consider(d, Hit::EnemyTank(ti));
                    }
                }
            } else if self.hp > 0.0 {
                let pa = Aabb {
                    center: self.tank.pos + Vector3::new(0.0, PLAYER_HIT_HALF.y, 0.0),
                    half: PLAYER_HIT_HALF,
                };
                if let Some(d) = pa.ray_hit(start, dir) {
                    consider(d, Hit::Player);
                }
            }

            let out_of_arena = end.x.abs() > ARENA_HALF || end.z.abs() > ARENA_HALF;
            if let Some((d, h)) = hit {
                let at = start + dir * d;
                self.explosions.push(Explosion {
                    pos: at,
                    age: 0.0,
                    duration: 0.45,
                    radius: 2.2,
                });
                match h {
                    Hit::Ground => self.sound(Sfx::Impact, at),
                    Hit::Crate(ti) => {
                        self.sound(Sfx::Impact, at);
                        let t = &mut self.targets[ti];
                        t.hp -= 1;
                        t.flash = 0.15;
                        if t.hp <= 0 {
                            t.respawn = Some(TARGET_RESPAWN);
                            let c = t.aabb.center;
                            if owner == Owner::Player {
                                self.crates_destroyed += 1;
                            }
                            self.explosions.push(Explosion {
                                pos: c,
                                age: 0.0,
                                duration: 0.8,
                                radius: 5.0,
                            });
                            self.burst(c, 14, 12.0, 1);
                            self.sound(Sfx::CrateBreak, c);
                        }
                    }
                    Hit::Enemy(ei) => {
                        let e = &mut self.enemies[ei];
                        e.hp -= 1;
                        e.flash = 0.15;
                        let destroyed = e.hp <= 0;
                        self.sound(
                            if destroyed {
                                Sfx::Explosion
                            } else {
                                Sfx::HitEnemy
                            },
                            at,
                        );
                        let e = &mut self.enemies[ei];
                        if destroyed {
                            e.respawn = Some(ENEMY_RESPAWN);
                            let c = e.aabb().center;
                            self.enemies_destroyed += 1;
                            self.explosions.push(Explosion {
                                pos: c,
                                age: 0.0,
                                duration: 1.0,
                                radius: 6.0,
                            });
                            self.burst(c, 22, 15.0, 0);
                            self.shake = self.shake.max(0.5);
                        }
                    }
                    Hit::EnemyTank(ti) => {
                        let t = &mut self.enemy_tanks[ti];
                        t.hp -= 1;
                        t.flash = 0.15;
                        let destroyed = t.hp <= 0;
                        self.sound(
                            if destroyed {
                                Sfx::Explosion
                            } else {
                                Sfx::HitEnemy
                            },
                            at,
                        );
                        let t = &mut self.enemy_tanks[ti];
                        if destroyed {
                            t.respawn = Some(ENEMY_TANK_RESPAWN);
                            let c = t.aabb().center;
                            self.enemies_destroyed += 1;
                            self.explosions.push(Explosion {
                                pos: c,
                                age: 0.0,
                                duration: 1.1,
                                radius: 7.0,
                            });
                            self.burst(c, 26, 16.0, 0);
                            self.shake = self.shake.max(0.6);
                        }
                    }
                    Hit::Player => {
                        self.hp = (self.hp - damage).max(0.0);
                        self.hurt_flash = 0.35;
                        let destroyed = self.hp <= 0.0;
                        self.sound(
                            if destroyed {
                                Sfx::Explosion
                            } else {
                                Sfx::HitArmor
                            },
                            at,
                        );
                        self.shake = self.shake.max(0.8);
                        if self.hp <= 0.0 {
                            let c = self.player_aabb().center;
                            self.explosions.push(Explosion {
                                pos: c,
                                age: 0.0,
                                duration: 1.2,
                                radius: 7.0,
                            });
                            self.burst(c, 30, 16.0, 0);
                            self.shake = 1.0;
                        }
                    }
                }
                self.shells.swap_remove(i);
            } else if self.shells[i].life <= 0.0 || out_of_arena {
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

    fn update_effects(&mut self, dt: f32) {
        for e in &mut self.explosions {
            e.age += dt;
        }
        self.explosions.retain(|e| e.age < e.duration);

        for d in &mut self.debris {
            d.vel.y -= GRAVITY * dt;
            d.pos += d.vel * dt;
            d.angle += d.spin * dt;
            if d.pos.y < d.size * 0.5 {
                d.pos.y = d.size * 0.5;
                d.vel.y = -d.vel.y * 0.35;
                d.vel.x *= 0.6;
                d.vel.z *= 0.6;
                d.spin *= 0.6;
            }
            d.life -= dt;
        }
        self.debris.retain(|d| d.life > 0.0);

        self.hurt_flash = (self.hurt_flash - dt).max(0.0);
        self.shake = (self.shake - dt * 1.8).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet_world(seed: u64) -> World {
        let mut w = World::new(seed);
        w.targets.clear();
        w.enemies.clear();
        w.enemy_tanks.clear();
        w.obstacles.clear();
        w
    }

    fn enemy_at(pos: Vector3) -> Enemy {
        Enemy {
            pos,
            yaw: 0.0,
            pitch: 0.0,
            hp: ENEMY_HP,
            reload: 0.0,
            flash: 0.0,
            respawn: None,
            next_barrel: false,
        }
    }

    #[test]
    fn shooting_a_crate_destroys_it() {
        let mut w = quiet_world(7);
        w.targets.push(Target {
            aabb: Aabb {
                center: Vector3::new(0.0, 1.5, 30.0),
                half: Vector3::new(2.0, 1.5, 2.0),
            },
            hp: TARGET_HP,
            flash: 0.0,
            respawn: None,
        });
        let fire = Controls {
            fire: true,
            ..Default::default()
        };
        for _ in 0..(60 * 4) {
            w.update(&fire, 1.0 / 60.0);
        }
        assert!(w.crates_destroyed >= 1);
        assert!(!w.debris.is_empty() || w.time > 3.0);
    }

    #[test]
    fn player_can_destroy_an_enemy() {
        let mut w = quiet_world(9);
        let mut e = enemy_at(Vector3::new(0.0, 0.0, 30.0));
        e.reload = 1000.0; // keep it from firing back
        w.enemies.push(e);
        let fire = Controls {
            fire: true,
            ..Default::default()
        };
        for _ in 0..(60 * 6) {
            w.update(&fire, 1.0 / 60.0);
            w.enemies[0].reload = 1000.0;
        }
        assert!(w.enemies_destroyed >= 1);
    }

    #[test]
    fn enemy_turns_and_hits_the_player() {
        let mut w = quiet_world(11);
        // Behind and to the side of the player, facing away at first.
        let mut e = enemy_at(Vector3::new(25.0, 0.0, -20.0));
        e.yaw = 0.0;
        w.enemies.push(e);
        let idle = Controls::default();
        for _ in 0..(60 * 15) {
            w.update(&idle, 1.0 / 60.0);
        }
        assert!(w.hp < PLAYER_MAX_HP, "hp {}", w.hp);
    }

    #[test]
    fn crate_blocks_enemy_fire() {
        let mut w = quiet_world(13);
        w.enemies.push(enemy_at(Vector3::new(0.0, 0.0, 40.0)));
        w.targets.push(Target {
            aabb: Aabb {
                center: Vector3::new(0.0, 3.0, 20.0),
                half: Vector3::new(4.0, 3.0, 1.0),
            },
            hp: 1000,
            flash: 0.0,
            respawn: None,
        });
        for _ in 0..(60 * 10) {
            w.update(&Controls::default(), 1.0 / 60.0);
        }
        assert_eq!(w.hp, PLAYER_MAX_HP);
    }

    #[test]
    fn dead_player_cannot_fire_and_can_restart() {
        let mut w = quiet_world(15);
        w.hp = 0.0;
        w.update(
            &Controls {
                fire: true,
                ..Default::default()
            },
            1.0 / 60.0,
        );
        assert!(w.shells.is_empty());
        w.update(
            &Controls {
                restart: true,
                ..Default::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(w.hp, PLAYER_MAX_HP);
    }

    #[test]
    fn tank_cannot_drive_through_target() {
        let mut w = quiet_world(3);
        w.targets.push(Target {
            aabb: Aabb {
                center: Vector3::new(0.0, 1.5, 10.0),
                half: Vector3::new(2.0, 1.5, 2.0),
            },
            hp: TARGET_HP,
            flash: 0.0,
            respawn: None,
        });
        let go = Controls {
            throttle: 1.0,
            ..Default::default()
        };
        for _ in 0..300 {
            w.update(&go, 1.0 / 60.0);
        }
        assert!(w.tank.pos.z < 10.0 - 2.0 - tank::COLLISION_RADIUS + 0.01);
    }

    #[test]
    fn enemy_tank_hunts_and_hits_the_player() {
        let mut w = quiet_world(17);
        w.enemy_tanks
            .push(EnemyTank::new(Vector3::new(30.0, 0.0, -60.0), 0.0));
        for _ in 0..(60 * 30) {
            w.update(&Controls::default(), 1.0 / 60.0);
        }
        assert!(w.hp < PLAYER_MAX_HP, "hp {}", w.hp);
        let t = &w.enemy_tanks[0];
        assert!(t.pos.distance(w.tank.pos) < 60.0, "{:?}", t.pos);
    }

    #[test]
    fn player_can_destroy_an_enemy_tank() {
        let mut w = quiet_world(19);
        w.enemy_tanks
            .push(EnemyTank::new(Vector3::new(0.0, 0.0, 30.0), 0.0));
        let fire = Controls {
            fire: true,
            ..Default::default()
        };
        for _ in 0..(60 * 8) {
            // Hold it still in front of the gun and keep it from firing back.
            let t = &mut w.enemy_tanks[0];
            t.pos = Vector3::new(0.0, 0.0, 30.0);
            t.reload = 1000.0;
            w.update(&fire, 1.0 / 60.0);
        }
        assert!(w.enemies_destroyed >= 1);
    }

    #[test]
    fn full_battle_stays_sane() {
        // A busy minute with everything spawned: tanks stay in the arena and apart.
        let mut w = World::new(21);
        let go = Controls {
            throttle: 0.5,
            steer: 0.3,
            fire: true,
            ..Default::default()
        };
        for _ in 0..(60 * 60) {
            w.update(&go, 1.0 / 60.0);
            for t in w.enemy_tanks.iter().filter(|t| t.alive()) {
                assert!(t.pos.x.is_finite() && t.pos.z.is_finite());
                assert!(t.pos.x.abs() <= ARENA_HALF && t.pos.z.abs() <= ARENA_HALF);
            }
        }
    }

    #[test]
    fn firing_and_hits_make_sounds() {
        let mut w = quiet_world(23);
        w.targets.push(Target {
            aabb: Aabb {
                center: Vector3::new(0.0, 1.5, 30.0),
                half: Vector3::new(2.0, 1.5, 2.0),
            },
            hp: TARGET_HP,
            flash: 0.0,
            respawn: None,
        });
        let fire = Controls {
            fire: true,
            ..Default::default()
        };
        let mut heard = Vec::new();
        for _ in 0..(60 * 4) {
            w.update(&fire, 1.0 / 60.0);
            heard.extend(w.sounds.iter().map(|s| s.sfx));
        }
        for want in [Sfx::PlayerCannon, Sfx::Impact, Sfx::CrateBreak, Sfx::Reload] {
            assert!(heard.contains(&want), "no {want:?} in {heard:?}");
        }
    }

    #[test]
    fn danger_rises_with_enemies_near() {
        let mut w = quiet_world(25);
        assert_eq!(w.danger(), 0.0);
        w.enemy_tanks
            .push(EnemyTank::new(Vector3::new(0.0, 0.0, 150.0), 0.0));
        assert_eq!(w.danger(), 0.0, "a far tank is no fight yet");
        w.enemy_tanks[0].pos = Vector3::new(0.0, 0.0, 40.0);
        assert!(w.danger() > 0.5);
        w.hp = 0.0;
        assert_eq!(w.danger(), 0.0);
    }
}
