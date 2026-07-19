mod render;

use std::{
    f32::consts::{LN_2, PI, TAU},
    mem,
    path::PathBuf,
};

use macroquad::math::{vec2, Vec2};

use crate::{
    enemies::{
        circuit_pose, Enemy, EnemyBullet, EnemyKind, COMMAND_MAX_FIRE_SECONDS,
        COMMAND_MIN_FIRE_SECONDS, DEATH_MAX_SPEED, DROID_SPEED, MINE_CAP,
    },
    game::{is_fleet_bonus_wave, next_extra_ship_threshold, wave_size, GameState, PlayPhase},
    hiscore::{self, DEFAULT_HIGH_SCORE},
    particles::{self, Particle},
    rng::Rng,
    vector::Seg,
};

pub const VIRTUAL_WIDTH: u32 = 1024;
pub const VIRTUAL_HEIGHT: u32 = 768;
pub const TICK_RATE: f32 = 60.0;
pub const TICK_SECONDS: f32 = 1.0 / TICK_RATE;

pub const OUTER_LEFT: f32 = 12.0;
pub const OUTER_TOP: f32 = 12.0;
pub const OUTER_RIGHT: f32 = VIRTUAL_WIDTH as f32 - 12.0;
pub const OUTER_BOTTOM: f32 = VIRTUAL_HEIGHT as f32 - 12.0;

pub const CONSOLE_LEFT: f32 = 252.0;
pub const CONSOLE_TOP: f32 = 288.0;
pub const CONSOLE_RIGHT: f32 = 772.0;
pub const CONSOLE_BOTTOM: f32 = 528.0;

const TURN_RATE: f32 = 330.0_f32.to_radians();
const TURN_EASE_SECONDS: f32 = 0.060;
const THRUST_ACCELERATION: f32 = 420.0;
const MAX_SPEED: f32 = 520.0;
const DRAG_HALF_LIFE: f32 = 2.5;
const RESTITUTION: f32 = 0.9;
const SHIP_RADIUS: f32 = 10.0;
const SHOT_SPEED: f32 = 900.0;
const SHOT_HALF_LENGTH: f32 = 5.5;
const ENEMY_BULLET_HALF_LENGTH: f32 = 5.0;
const MAX_SHOTS: usize = 4;
const BORDER_FLASH_SECONDS: f32 = 0.3;
const BORDER_NEIGHBOUR_FLASH_STRENGTH: f32 = 0.3;
const BORDER_IDLE_INTENSITY: f32 = 0.15;
const SCORE_FLASH_SECONDS: f32 = 0.15;
const SPAWN_WARNING_SECONDS: f32 = 1.5;
const WAVE_CLEARED_SECONDS: f32 = 2.0;
const FLEET_BONUS_SECONDS: f32 = 2.5;
const SHIP_DEATH_SECONDS: f32 = 1.5;
const GAME_OVER_SECONDS: f32 = 7.0;
const RESPAWN_CLEARANCE: f32 = 140.0;
const ESCALATION_FIRST_SECONDS: f32 = 10.0;
const ESCALATION_REPEAT_SECONDS: f32 = 8.0;
const COMMAND_PROMOTION_SECONDS: f32 = 12.0;
const LONE_PROMOTION_SECONDS: f32 = 3.0;
const FLEET_BONUS_POINTS: u32 = 5_000;

const OUTER_TOP_EDGE: usize = 0;
const OUTER_RIGHT_EDGE: usize = 1;
const OUTER_BOTTOM_EDGE: usize = 2;
const OUTER_LEFT_EDGE: usize = 3;
const CONSOLE_TOP_EDGE: usize = 4;
const CONSOLE_RIGHT_EDGE: usize = 5;
const CONSOLE_BOTTOM_EDGE: usize = 6;
const CONSOLE_LEFT_EDGE: usize = 7;

const fn shape_seg(x1: f32, y1: f32, x2: f32, y2: f32) -> Seg {
    Seg::new(Vec2::new(x1, y1), Vec2::new(x2, y2), 1.0)
}

pub const SHIP_SHAPE: &[Seg] = &[
    shape_seg(14.0, 0.0, -9.0, -7.0),
    shape_seg(-9.0, -7.0, -5.0, 0.0),
    shape_seg(-5.0, 0.0, -9.0, 7.0),
    shape_seg(-9.0, 7.0, 14.0, 0.0),
    shape_seg(-5.0, 0.0, 10.0, 0.0),
    shape_seg(-9.0, -7.0, -9.0, 7.0),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub thrust: bool,
    pub fire: bool,
    pub start: bool,
    pub pause: bool,
    pub escape: bool,
}

impl InputState {
    pub fn union(self, other: Self) -> Self {
        Self {
            left: self.left || other.left,
            right: self.right || other.right,
            thrust: self.thrust || other.thrust,
            fire: self.fire || other.fire,
            start: self.start || other.start,
            pause: self.pause || other.pause,
            escape: self.escape || other.escape,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub angular_velocity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub position: Vec2,
    pub velocity: Vec2,
}

#[derive(Clone, Debug)]
pub struct Simulation {
    pub player: Player,
    shots: Vec<Shot>,
    enemies: Vec<Enemy>,
    enemy_bullets: Vec<EnemyBullet>,
    particles: Vec<Particle>,
    border_flash: [f32; 8],
    rng: Rng,
    seed: u64,
    frame: u64,
    state_frame: u64,
    previous_fire: bool,
    previous_start: bool,
    previous_pause: bool,
    previous_escape: bool,
    paused: bool,
    quit_requested: bool,
    thrusting: bool,
    exhaust_length: f32,
    exhaust_spread: f32,
    state: GameState,
    play_phase: PlayPhase,
    phase_timer: f32,
    death_timer: f32,
    game_over_timer: f32,
    wave_age: f32,
    next_escalation: f32,
    mine_drop_timer: f32,
    lone_timer: f32,
    last_ship_count: usize,
    convoy_direction: f32,
    score: u32,
    score_flash: f32,
    high_score: u32,
    ships: u32,
    wave: u32,
    next_extra_ship: u32,
    extra_ship_flash: f32,
    new_high_score: bool,
    high_score_path: Option<PathBuf>,
}

impl Simulation {
    /// Deterministic, persistence-free simulation used by headless mode and tests.
    pub fn new(seed: u64) -> Self {
        Self::with_high_score(seed, DEFAULT_HIGH_SCORE, None)
    }

    /// Windowed simulation. This is the only constructor that reads high-score data.
    pub fn persistent(seed: u64) -> Self {
        let path = hiscore::path_from_environment();
        let high_score = hiscore::load(&path).unwrap_or(DEFAULT_HIGH_SCORE);
        Self::with_high_score(seed, high_score, Some(path))
    }

    fn with_high_score(seed: u64, high_score: u32, high_score_path: Option<PathBuf>) -> Self {
        Self {
            player: spawn_player(),
            shots: Vec::with_capacity(MAX_SHOTS),
            enemies: Vec::new(),
            enemy_bullets: Vec::new(),
            particles: Vec::new(),
            border_flash: [0.0; 8],
            rng: Rng::new(seed),
            seed,
            frame: 0,
            state_frame: 0,
            previous_fire: false,
            previous_start: false,
            previous_pause: false,
            previous_escape: false,
            paused: false,
            quit_requested: false,
            thrusting: false,
            exhaust_length: 12.0,
            exhaust_spread: 3.0,
            state: GameState::Attract,
            play_phase: PlayPhase::Warning,
            phase_timer: 0.0,
            death_timer: 0.0,
            game_over_timer: 0.0,
            wave_age: 0.0,
            next_escalation: ESCALATION_FIRST_SECONDS,
            mine_drop_timer: 3.4,
            lone_timer: LONE_PROMOTION_SECONDS,
            last_ship_count: 0,
            convoy_direction: 1.0,
            score: 0,
            score_flash: 0.0,
            high_score,
            ships: 3,
            wave: 1,
            next_extra_ship: 40_000,
            extra_ship_flash: 0.0,
            new_high_score: false,
            high_score_path,
        }
    }

    pub fn tick(&mut self, input: InputState) {
        let start_pressed = input.start && !self.previous_start;
        let pause_pressed = input.pause && !self.previous_pause;
        let escape_pressed = input.escape && !self.previous_escape;

        self.previous_start = input.start;
        self.previous_pause = input.pause;
        self.previous_escape = input.escape;

        if escape_pressed {
            if self.state == GameState::Attract {
                self.quit_requested = true;
            } else {
                self.enter_attract();
            }
        }

        if start_pressed {
            match self.state {
                GameState::Attract => self.start_game(),
                GameState::GameOver => self.enter_attract(),
                GameState::Playing | GameState::ShipDeath => {}
            }
        }

        if pause_pressed && matches!(self.state, GameState::Playing | GameState::ShipDeath) {
            self.paused = !self.paused;
        }
        if self.paused {
            self.previous_fire = input.fire;
            self.thrusting = false;
            self.frame = self.frame.wrapping_add(1);
            return;
        }

        for timer in &mut self.border_flash {
            *timer = (*timer - TICK_SECONDS).max(0.0);
        }
        self.extra_ship_flash = (self.extra_ship_flash - TICK_SECONDS).max(0.0);
        self.score_flash = (self.score_flash - TICK_SECONDS).max(0.0);

        match self.state {
            GameState::Attract => {}
            GameState::Playing => self.update_playing(input),
            GameState::ShipDeath => self.update_ship_death(),
            GameState::GameOver => self.update_game_over(),
        }

        self.previous_fire = input.fire;
        self.frame = self.frame.wrapping_add(1);
        self.state_frame = self.state_frame.wrapping_add(1);
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    #[cfg(test)]
    pub fn shot_count(&self) -> usize {
        self.shots.len()
    }

    fn start_game(&mut self) {
        self.rng = Rng::new(self.seed);
        self.player = spawn_player();
        self.shots.clear();
        self.enemies.clear();
        self.enemy_bullets.clear();
        self.particles.clear();
        self.border_flash = [0.0; 8];
        self.state = GameState::Playing;
        self.state_frame = 0;
        self.paused = false;
        self.thrusting = false;
        self.score = 0;
        self.score_flash = 0.0;
        self.ships = 3;
        self.wave = 1;
        self.next_extra_ship = 40_000;
        self.extra_ship_flash = 0.0;
        self.new_high_score = false;
        self.spawn_wave();
    }

    fn enter_attract(&mut self) {
        self.state = GameState::Attract;
        self.state_frame = 0;
        self.paused = false;
        self.thrusting = false;
        self.shots.clear();
        self.enemies.clear();
        self.enemy_bullets.clear();
        self.particles.clear();
    }

    fn enter_game_over(&mut self) {
        self.state = GameState::GameOver;
        self.state_frame = 0;
        self.game_over_timer = GAME_OVER_SECONDS;
        self.paused = false;
        self.thrusting = false;
        self.shots.clear();
        self.enemies.clear();
        self.enemy_bullets.clear();
        self.new_high_score = self.score > self.high_score;
        if self.new_high_score {
            self.high_score = self.score;
            if let Some(path) = &self.high_score_path {
                let _ = hiscore::save(path, self.high_score);
            }
        }
    }

    fn update_game_over(&mut self) {
        particles::update(&mut self.particles, TICK_SECONDS);
        self.game_over_timer -= TICK_SECONDS;
        if self.game_over_timer <= 0.0 {
            self.enter_attract();
        }
    }

    fn update_playing(&mut self, input: InputState) {
        match self.play_phase {
            PlayPhase::Warning => {
                self.update_player(input);
                self.update_shots_walls();
                self.age_mines();
                self.handle_combat();
                particles::update(&mut self.particles, TICK_SECONDS);
                self.phase_timer -= TICK_SECONDS;
                if self.phase_timer <= 0.0 {
                    self.play_phase = PlayPhase::Active;
                    self.wave_age = 0.0;
                    self.next_escalation = ESCALATION_FIRST_SECONDS;
                    self.mine_drop_timer = self.rng.range_f32(3.2, 4.8);
                }
            }
            PlayPhase::Active => {
                self.update_player(input);
                self.update_shots_walls();
                self.update_enemies();
                self.update_enemy_bullets();
                self.update_escalation();
                self.update_droid_mine_drop();
                self.handle_combat();
                if self.state == GameState::Playing && self.ship_count() == 0 {
                    self.complete_wave();
                }
                particles::update(&mut self.particles, TICK_SECONDS);
            }
            PlayPhase::WaveCleared | PlayPhase::FleetBonus => {
                self.update_player(input);
                self.update_shots_walls();
                self.age_mines();
                self.handle_combat();
                particles::update(&mut self.particles, TICK_SECONDS);
                self.phase_timer -= TICK_SECONDS;
                if self.phase_timer <= 0.0 {
                    self.wave = self.wave.saturating_add(1);
                    self.spawn_wave();
                }
            }
        }
    }

    fn update_ship_death(&mut self) {
        if self.play_phase == PlayPhase::Active {
            self.update_enemies();
            self.update_enemy_bullets();
            self.update_escalation();
            self.update_droid_mine_drop();
            if self.ship_count() == 0 {
                self.complete_wave();
            }
        } else {
            self.age_mines();
        }
        particles::update(&mut self.particles, TICK_SECONDS);
        self.death_timer -= TICK_SECONDS;
        if self.death_timer > 0.0 {
            return;
        }
        if self.ships == 0 {
            self.enter_game_over();
        } else if self.spawn_is_safe() {
            self.player = spawn_player();
            self.state = GameState::Playing;
            self.state_frame = 0;
        }
    }

    fn update_player(&mut self, input: InputState) {
        let turn_direction = input.right as i8 as f32 - input.left as i8 as f32;
        let target_turn_rate = turn_direction * TURN_RATE;
        let maximum_turn_change = TURN_RATE / TURN_EASE_SECONDS * TICK_SECONDS;
        self.player.angular_velocity = approach(
            self.player.angular_velocity,
            target_turn_rate,
            maximum_turn_change,
        );
        self.player.rotation =
            (self.player.rotation + self.player.angular_velocity * TICK_SECONDS).rem_euclid(TAU);

        let facing = direction(self.player.rotation);
        self.thrusting = input.thrust;
        if input.thrust {
            self.player.velocity += facing * THRUST_ACCELERATION * TICK_SECONDS;
            if self.frame.is_multiple_of(2) {
                let speed_extension = (self.player.velocity.length() / MAX_SPEED).min(1.0) * 10.0;
                self.exhaust_length = self.rng.range_f32(9.0, 16.0) + speed_extension;
                self.exhaust_spread = self.rng.range_f32(2.0, 5.0);
            }
        }

        let drag = (-LN_2 * TICK_SECONDS / DRAG_HALF_LIFE).exp();
        self.player.velocity *= drag;
        self.player.velocity = self.player.velocity.clamp_length_max(MAX_SPEED);
        self.player.position += self.player.velocity * TICK_SECONDS;
        resolve_circle_arena(
            &mut self.player.position,
            &mut self.player.velocity,
            SHIP_RADIUS,
            &mut self.border_flash,
        );

        if input.fire && !self.previous_fire && self.shots.len() < MAX_SHOTS {
            self.shots.push(Shot {
                position: self.player.position + facing * 17.0,
                velocity: facing * SHOT_SPEED,
            });
        }
    }

    fn update_shots_walls(&mut self) {
        let mut flashes = [false; 8];
        self.shots.retain_mut(|shot| {
            shot.position += shot.velocity * TICK_SECONDS;
            if let Some(edge) = projectile_collision_edge(shot.position) {
                flashes[edge] = true;
                false
            } else {
                true
            }
        });
        apply_flashes(&mut self.border_flash, flashes);
    }

    fn update_enemy_bullets(&mut self) {
        let mut flashes = [false; 8];
        self.enemy_bullets.retain_mut(|bullet| {
            bullet.position += bullet.velocity * TICK_SECONDS;
            if let Some(edge) = projectile_collision_edge(bullet.position) {
                flashes[edge] = true;
                false
            } else {
                true
            }
        });
        apply_flashes(&mut self.border_flash, flashes);
    }

    fn update_enemies(&mut self) {
        let player_position = self.player.position;
        let convoy_direction = self.convoy_direction;
        let mut new_bullets = Vec::new();
        let mut vapor_mines = Vec::new();

        for enemy in &mut self.enemies {
            enemy.age += TICK_SECONDS;
            match enemy.kind {
                EnemyKind::PhotonMine | EnemyKind::VaporMine => {}
                EnemyKind::Droid => {
                    enemy.path_distance += convoy_direction * DROID_SPEED * TICK_SECONDS;
                    let (path_position, tangent, normal) = circuit_pose(enemy.path_distance);
                    let loose_jitter =
                        enemy.jitter + (enemy.age * 1.7 + enemy.wander_phase).sin() * 3.0;
                    let next_position = path_position + normal * loose_jitter;
                    enemy.velocity = (next_position - enemy.position) / TICK_SECONDS;
                    enemy.position = next_position;
                    if convoy_direction < 0.0 {
                        enemy.velocity = -tangent * DROID_SPEED
                            + normal * (enemy.age * 1.7 + enemy.wander_phase).cos() * 5.1;
                    }
                    enemy.rotation = enemy.velocity.y.atan2(enemy.velocity.x);
                }
                EnemyKind::Command => {
                    let speed = enemy.velocity.length().clamp(145.0, 205.0);
                    let steering = (enemy.age * 0.73 + enemy.wander_phase).sin() * 0.52;
                    let angle = enemy.velocity.y.atan2(enemy.velocity.x) + steering * TICK_SECONDS;
                    enemy.velocity = direction(angle) * speed;
                    enemy.position += enemy.velocity * TICK_SECONDS;
                    resolve_circle_arena(
                        &mut enemy.position,
                        &mut enemy.velocity,
                        enemy.kind.radius(),
                        &mut self.border_flash,
                    );
                    enemy.rotation = enemy.velocity.y.atan2(enemy.velocity.x);

                    enemy.action_timer -= TICK_SECONDS;
                    if enemy.action_timer <= 0.0 {
                        let aim_error = self
                            .rng
                            .range_f32(-10.0_f32.to_radians(), 10.0_f32.to_radians());
                        let aim = (player_position - enemy.position).normalize_or_zero();
                        let angle = aim.y.atan2(aim.x) + aim_error;
                        new_bullets.push(EnemyBullet {
                            position: enemy.position + direction(angle) * 15.0,
                            velocity: direction(angle) * 340.0,
                        });
                        enemy.action_timer = self
                            .rng
                            .range_f32(COMMAND_MIN_FIRE_SECONDS, COMMAND_MAX_FIRE_SECONDS);
                    }

                    enemy.mine_timer -= TICK_SECONDS;
                    if enemy.mine_timer <= 0.0 {
                        vapor_mines.push((
                            enemy.position - enemy.velocity.normalize_or_zero() * 18.0,
                            enemy.rotation,
                        ));
                        enemy.mine_timer = self.rng.range_f32(5.5, 8.5);
                    }
                }
                EnemyKind::Death => {
                    let desired = (player_position - enemy.position).normalize_or_zero();
                    enemy.velocity += desired * 230.0 * TICK_SECONDS;
                    enemy.velocity = enemy.velocity.clamp_length_max(DEATH_MAX_SPEED);
                    enemy.position += enemy.velocity * TICK_SECONDS;
                    resolve_circle_arena(
                        &mut enemy.position,
                        &mut enemy.velocity,
                        enemy.kind.radius(),
                        &mut self.border_flash,
                    );
                    enemy.rotation = enemy.velocity.y.atan2(enemy.velocity.x);
                }
            }
        }
        self.enemy_bullets.extend(new_bullets);
        for (position, rotation) in vapor_mines {
            self.try_spawn_mine(EnemyKind::VaporMine, position, rotation);
        }
    }

    fn age_mines(&mut self) {
        for enemy in &mut self.enemies {
            if enemy.kind.is_mine() {
                enemy.age += TICK_SECONDS;
            }
        }
    }

    fn update_escalation(&mut self) {
        self.wave_age += TICK_SECONDS;
        if self.wave_age >= self.next_escalation {
            self.promote_random_droid();
            self.next_escalation += ESCALATION_REPEAT_SECONDS;
        }

        let aged_commands: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| {
                (enemy.kind == EnemyKind::Command && enemy.age >= COMMAND_PROMOTION_SECONDS)
                    .then_some(index)
            })
            .collect();
        for index in aged_commands {
            self.promote_to_death(index);
        }

        let ship_count = self.ship_count();
        if ship_count == 1 {
            if self.last_ship_count != 1 {
                self.lone_timer = LONE_PROMOTION_SECONDS;
            } else {
                self.lone_timer -= TICK_SECONDS;
            }
            if self.lone_timer <= 0.0 {
                if let Some(index) = self.enemies.iter().position(|enemy| enemy.kind.is_ship()) {
                    match self.enemies[index].kind {
                        EnemyKind::Droid => self.promote_to_command(index),
                        EnemyKind::Command => self.promote_to_death(index),
                        EnemyKind::Death | EnemyKind::PhotonMine | EnemyKind::VaporMine => {}
                    }
                }
                self.lone_timer = LONE_PROMOTION_SECONDS;
            }
        } else {
            self.lone_timer = LONE_PROMOTION_SECONDS;
        }
        self.last_ship_count = ship_count;
    }

    fn promote_random_droid(&mut self) {
        let candidates: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| (enemy.kind == EnemyKind::Droid).then_some(index))
            .collect();
        if !candidates.is_empty() {
            let choice = self.rng.next_u64() as usize % candidates.len();
            self.promote_to_command(candidates[choice]);
        }
    }

    fn promote_to_command(&mut self, index: usize) {
        let enemy = &mut self.enemies[index];
        if enemy.kind != EnemyKind::Droid {
            return;
        }
        enemy.kind = EnemyKind::Command;
        enemy.age = 0.0;
        enemy.velocity = enemy.velocity.normalize_or_zero() * 175.0;
        enemy.action_timer = self
            .rng
            .range_f32(COMMAND_MIN_FIRE_SECONDS, COMMAND_MAX_FIRE_SECONDS);
        enemy.mine_timer = self.rng.range_f32(5.0, 7.0);
        enemy.wander_phase = self.rng.range_f32(0.0, TAU);
    }

    fn promote_to_death(&mut self, index: usize) {
        let enemy = &mut self.enemies[index];
        if enemy.kind != EnemyKind::Command {
            return;
        }
        enemy.kind = EnemyKind::Death;
        enemy.age = 0.0;
        enemy.velocity = enemy.velocity.normalize_or_zero() * 220.0;
    }

    fn update_droid_mine_drop(&mut self) {
        self.mine_drop_timer -= TICK_SECONDS;
        if self.mine_drop_timer > 0.0 {
            return;
        }
        self.mine_drop_timer = self.rng.range_f32(3.2, 5.0);
        let droids: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| (enemy.kind == EnemyKind::Droid).then_some(index))
            .collect();
        if droids.is_empty() {
            return;
        }
        let choice = self.rng.next_u64() as usize % droids.len();
        let droid = &self.enemies[droids[choice]];
        let position = droid.position - droid.velocity.normalize_or_zero() * 17.0;
        self.try_spawn_mine(EnemyKind::PhotonMine, position, droid.rotation);
    }

    fn try_spawn_mine(&mut self, kind: EnemyKind, position: Vec2, rotation: f32) -> bool {
        if self.mine_count() >= MINE_CAP
            || position.distance(self.player.position) < RESPAWN_CLEARANCE - 20.0
        {
            return false;
        }
        self.enemies.push(Enemy::mine(kind, position, rotation));
        true
    }

    fn handle_combat(&mut self) {
        let mut shot_dead = vec![false; self.shots.len()];
        let mut enemy_dead = vec![false; self.enemies.len()];
        for (shot_index, shot) in self.shots.iter().enumerate() {
            let previous = shot.position - shot.velocity * TICK_SECONDS;
            for (enemy_index, enemy) in self.enemies.iter().enumerate() {
                if enemy.kind.is_ship() && self.play_phase != PlayPhase::Active {
                    continue;
                }
                if segment_circle_hit(previous, shot.position, enemy.position, enemy.kind.radius())
                {
                    shot_dead[shot_index] = true;
                    enemy_dead[enemy_index] = true;
                    break;
                }
            }
        }

        let old_shots = mem::take(&mut self.shots);
        self.shots = old_shots
            .into_iter()
            .enumerate()
            .filter_map(|(index, shot)| (!shot_dead[index]).then_some(shot))
            .collect();

        let old_enemies = mem::take(&mut self.enemies);
        for (index, enemy) in old_enemies.into_iter().enumerate() {
            if enemy_dead[index] {
                self.spawn_enemy_shatter(&enemy, 0.9, 115.0);
                self.award_points(enemy.kind.points());
            } else {
                self.enemies.push(enemy);
            }
        }

        if self.state != GameState::Playing {
            return;
        }
        let enemy_contact = self.enemies.iter().any(|enemy| {
            (!enemy.kind.is_ship() || self.play_phase == PlayPhase::Active)
                && self.player.position.distance(enemy.position)
                    <= SHIP_RADIUS + enemy.kind.radius()
        });
        let bullet_hit = self.enemy_bullets.iter().position(|bullet| {
            let previous = bullet.position - bullet.velocity * TICK_SECONDS;
            segment_circle_hit(previous, bullet.position, self.player.position, SHIP_RADIUS)
        });
        if let Some(index) = bullet_hit {
            self.enemy_bullets.swap_remove(index);
        }
        if enemy_contact || bullet_hit.is_some() {
            self.kill_player();
        }
    }

    fn kill_player(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        particles::spawn_shatter(
            &mut self.particles,
            particles::ShatterSpec {
                shape: SHIP_SHAPE,
                position: self.player.position,
                rotation: self.player.rotation,
                scale: 1.0,
                base_velocity: self.player.velocity,
                lifetime: 1.4,
                energy: 215.0,
            },
            &mut self.rng,
        );
        self.ships = self.ships.saturating_sub(1);
        self.state = GameState::ShipDeath;
        self.state_frame = 0;
        self.death_timer = SHIP_DEATH_SECONDS;
        self.thrusting = false;
        self.shots.clear();
    }

    fn spawn_is_safe(&self) -> bool {
        let spawn = spawn_position();
        self.enemies
            .iter()
            .filter(|enemy| enemy.kind.is_ship())
            .all(|enemy| enemy.position.distance(spawn) >= RESPAWN_CLEARANCE)
            && self
                .enemy_bullets
                .iter()
                .all(|bullet| bullet.position.distance(spawn) >= RESPAWN_CLEARANCE)
    }

    fn spawn_enemy_shatter(&mut self, enemy: &Enemy, lifetime: f32, energy: f32) {
        particles::spawn_shatter(
            &mut self.particles,
            particles::ShatterSpec {
                shape: enemy.kind.shape(),
                position: enemy.position,
                rotation: enemy.rotation,
                scale: 1.0,
                base_velocity: enemy.velocity,
                lifetime,
                energy,
            },
            &mut self.rng,
        );
    }

    fn complete_wave(&mut self) {
        self.enemy_bullets.clear();
        if is_fleet_bonus_wave(self.wave) {
            let old_enemies = mem::take(&mut self.enemies);
            for enemy in old_enemies {
                if enemy.kind.is_mine() {
                    self.spawn_enemy_shatter(&enemy, 0.5, 42.0);
                } else {
                    self.enemies.push(enemy);
                }
            }
            self.award_points(FLEET_BONUS_POINTS);
            self.play_phase = PlayPhase::FleetBonus;
            self.phase_timer = FLEET_BONUS_SECONDS;
        } else {
            self.play_phase = PlayPhase::WaveCleared;
            self.phase_timer = WAVE_CLEARED_SECONDS;
        }
    }

    fn award_points(&mut self, points: u32) {
        self.score = self.score.saturating_add(points);
        self.score_flash = SCORE_FLASH_SECONDS;
        while self.score >= self.next_extra_ship {
            self.ships = self.ships.saturating_add(1);
            self.extra_ship_flash = 2.0;
            self.next_extra_ship = if self.next_extra_ship == 40_000 {
                100_000
            } else {
                self.next_extra_ship.saturating_add(100_000)
            };
        }
        debug_assert_eq!(self.next_extra_ship, next_extra_ship_threshold(self.score));
    }

    fn spawn_wave(&mut self) {
        self.enemies.retain(|enemy| enemy.kind.is_mine());
        self.enemy_bullets.clear();
        self.shots.clear();
        self.convoy_direction = if self.rng.next_f32() < 0.5 { -1.0 } else { 1.0 };
        let count = wave_size(self.wave);
        for index in 0..count {
            let distance = 72.0 + index as f32 * 58.0;
            let jitter = self.rng.range_f32(-9.0, 9.0);
            self.enemies
                .push(Enemy::droid(distance, jitter, self.convoy_direction));
        }
        self.play_phase = PlayPhase::Warning;
        self.phase_timer = SPAWN_WARNING_SECONDS;
        self.wave_age = 0.0;
        self.next_escalation = ESCALATION_FIRST_SECONDS;
        self.mine_drop_timer = 3.4;
        self.lone_timer = LONE_PROMOTION_SECONDS;
        self.last_ship_count = count;
    }

    fn ship_count(&self) -> usize {
        self.enemies
            .iter()
            .filter(|enemy| enemy.kind.is_ship())
            .count()
    }

    fn mine_count(&self) -> usize {
        self.enemies
            .iter()
            .filter(|enemy| enemy.kind.is_mine())
            .count()
    }
}

pub fn reflect_velocity(velocity: Vec2, normal: Vec2, restitution: f32) -> Vec2 {
    let normal = normal.normalize_or_zero();
    (velocity - 2.0 * velocity.dot(normal) * normal) * restitution
}

fn spawn_position() -> Vec2 {
    vec2(512.0, (CONSOLE_BOTTOM + OUTER_BOTTOM) * 0.5)
}

fn spawn_player() -> Player {
    Player {
        position: spawn_position(),
        velocity: Vec2::ZERO,
        rotation: -PI * 0.5,
        angular_velocity: 0.0,
    }
}

fn direction(rotation: f32) -> Vec2 {
    vec2(rotation.cos(), rotation.sin())
}

fn approach(current: f32, target: f32, maximum_change: f32) -> f32 {
    current + (target - current).clamp(-maximum_change, maximum_change)
}

fn resolve_circle_arena(
    position: &mut Vec2,
    velocity: &mut Vec2,
    radius: f32,
    border_flash: &mut [f32; 8],
) {
    if resolve_axis_wall(
        &mut position.x,
        OUTER_LEFT + radius,
        true,
        velocity,
        vec2(1.0, 0.0),
    ) {
        flash_border_edge(border_flash, OUTER_LEFT_EDGE);
    }
    if resolve_axis_wall(
        &mut position.x,
        OUTER_RIGHT - radius,
        false,
        velocity,
        vec2(-1.0, 0.0),
    ) {
        flash_border_edge(border_flash, OUTER_RIGHT_EDGE);
    }
    if resolve_axis_wall(
        &mut position.y,
        OUTER_TOP + radius,
        true,
        velocity,
        vec2(0.0, 1.0),
    ) {
        flash_border_edge(border_flash, OUTER_TOP_EDGE);
    }
    if resolve_axis_wall(
        &mut position.y,
        OUTER_BOTTOM - radius,
        false,
        velocity,
        vec2(0.0, -1.0),
    ) {
        flash_border_edge(border_flash, OUTER_BOTTOM_EDGE);
    }

    let expanded_left = CONSOLE_LEFT - radius;
    let expanded_right = CONSOLE_RIGHT + radius;
    let expanded_top = CONSOLE_TOP - radius;
    let expanded_bottom = CONSOLE_BOTTOM + radius;
    if position.x > expanded_left
        && position.x < expanded_right
        && position.y > expanded_top
        && position.y < expanded_bottom
    {
        let distances = [
            (position.y - expanded_top, CONSOLE_TOP_EDGE),
            (expanded_right - position.x, CONSOLE_RIGHT_EDGE),
            (expanded_bottom - position.y, CONSOLE_BOTTOM_EDGE),
            (position.x - expanded_left, CONSOLE_LEFT_EDGE),
        ];
        let (_, edge) = distances
            .into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .expect("console has four edges");
        let (normal, corrected_position) = match edge {
            CONSOLE_TOP_EDGE => (vec2(0.0, -1.0), vec2(position.x, expanded_top)),
            CONSOLE_RIGHT_EDGE => (vec2(1.0, 0.0), vec2(expanded_right, position.y)),
            CONSOLE_BOTTOM_EDGE => (vec2(0.0, 1.0), vec2(position.x, expanded_bottom)),
            CONSOLE_LEFT_EDGE => (vec2(-1.0, 0.0), vec2(expanded_left, position.y)),
            _ => unreachable!(),
        };
        *position = corrected_position;
        if velocity.dot(normal) < 0.0 {
            *velocity = reflect_velocity(*velocity, normal, RESTITUTION);
            flash_border_edge(border_flash, edge);
        }
    }
}

fn resolve_axis_wall(
    coordinate: &mut f32,
    limit: f32,
    is_minimum: bool,
    velocity: &mut Vec2,
    inward_normal: Vec2,
) -> bool {
    let crossed = if is_minimum {
        *coordinate < limit
    } else {
        *coordinate > limit
    };
    if crossed {
        *coordinate = limit;
        if velocity.dot(inward_normal) < 0.0 {
            *velocity = reflect_velocity(*velocity, inward_normal, RESTITUTION);
            return true;
        }
    }
    false
}

fn projectile_collision_edge(position: Vec2) -> Option<usize> {
    if position.y <= OUTER_TOP {
        return Some(OUTER_TOP_EDGE);
    }
    if position.x >= OUTER_RIGHT {
        return Some(OUTER_RIGHT_EDGE);
    }
    if position.y >= OUTER_BOTTOM {
        return Some(OUTER_BOTTOM_EDGE);
    }
    if position.x <= OUTER_LEFT {
        return Some(OUTER_LEFT_EDGE);
    }

    if position.x >= CONSOLE_LEFT
        && position.x <= CONSOLE_RIGHT
        && position.y >= CONSOLE_TOP
        && position.y <= CONSOLE_BOTTOM
    {
        let distances = [
            (position.y - CONSOLE_TOP, CONSOLE_TOP_EDGE),
            (CONSOLE_RIGHT - position.x, CONSOLE_RIGHT_EDGE),
            (CONSOLE_BOTTOM - position.y, CONSOLE_BOTTOM_EDGE),
            (position.x - CONSOLE_LEFT, CONSOLE_LEFT_EDGE),
        ];
        return distances
            .into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, edge)| edge);
    }
    None
}

fn segment_circle_hit(a: Vec2, b: Vec2, center: Vec2, radius: f32) -> bool {
    let segment = b - a;
    let length_squared = segment.length_squared();
    if length_squared <= f32::EPSILON {
        return a.distance_squared(center) <= radius * radius;
    }
    let projection = ((center - a).dot(segment) / length_squared).clamp(0.0, 1.0);
    let nearest = a + segment * projection;
    nearest.distance_squared(center) <= radius * radius
}

fn apply_flashes(border_flash: &mut [f32; 8], flashes: [bool; 8]) {
    for (edge, hit) in flashes.into_iter().enumerate() {
        if hit {
            flash_border_edge(border_flash, edge);
        }
    }
}

fn flash_border_edge(border_flash: &mut [f32; 8], edge: usize) {
    let group_start = edge / 4 * 4;
    let local_edge = edge % 4;
    let neighbour_flash = BORDER_FLASH_SECONDS * BORDER_NEIGHBOUR_FLASH_STRENGTH;
    for neighbour in [(local_edge + 3) % 4, (local_edge + 1) % 4] {
        let timer = &mut border_flash[group_start + neighbour];
        *timer = timer.max(neighbour_flash);
    }
    border_flash[edge] = BORDER_FLASH_SECONDS;
}

#[cfg(test)]
mod tests {
    use macroquad::math::vec2;

    use super::{
        reflect_velocity, Enemy, EnemyKind, GameState, InputState, PlayPhase, Simulation,
        TICK_SECONDS,
    };
    use crate::enemies::MINE_CAP;

    fn start(simulation: &mut Simulation) {
        simulation.tick(InputState {
            start: true,
            ..InputState::default()
        });
        simulation.tick(InputState::default());
    }

    #[test]
    fn reflection_changes_direction_and_scales_speed_by_restitution() {
        let velocity = vec2(-30.0, 40.0);
        let reflected = reflect_velocity(velocity, vec2(1.0, 0.0), 0.9);
        assert!((reflected - vec2(27.0, 36.0)).length() < 0.0001);
        assert!((reflected.length() - velocity.length() * 0.9).abs() < 0.0001);
    }

    #[test]
    fn border_flash_spills_to_adjacent_edges_at_reduced_strength() {
        let mut flashes = [0.0; 8];
        super::flash_border_edge(&mut flashes, super::OUTER_TOP_EDGE);
        assert_eq!(flashes[super::OUTER_TOP_EDGE], super::BORDER_FLASH_SECONDS);
        let neighbour = super::BORDER_FLASH_SECONDS * super::BORDER_NEIGHBOUR_FLASH_STRENGTH;
        assert_eq!(flashes[super::OUTER_LEFT_EDGE], neighbour);
        assert_eq!(flashes[super::OUTER_RIGHT_EDGE], neighbour);
        assert_eq!(flashes[super::OUTER_BOTTOM_EDGE], 0.0);
        assert!(flashes[4..].iter().all(|timer| *timer == 0.0));
    }

    #[test]
    fn only_four_shots_can_be_live() {
        let mut simulation = Simulation::new(7);
        start(&mut simulation);
        simulation.player.rotation = 0.0;
        for _ in 0..6 {
            simulation.tick(InputState {
                fire: true,
                ..InputState::default()
            });
            simulation.tick(InputState::default());
        }
        assert_eq!(simulation.shot_count(), 4);
    }

    #[test]
    fn equal_seed_and_start_play_input_produce_identical_display_lists() {
        fn run(seed: u64) -> crate::vector::DisplayList {
            let mut simulation = Simulation::new(seed);
            for frame in 0..360 {
                simulation.tick(InputState {
                    start: frame == 0,
                    left: (20..75).contains(&frame),
                    right: (130..190).contains(&frame),
                    thrust: (10..170).contains(&frame),
                    fire: matches!(frame, 30 | 60 | 90 | 120 | 180),
                    ..InputState::default()
                });
            }
            simulation.display_list()
        }

        assert_eq!(run(0x1234_5678), run(0x1234_5678));
    }

    #[test]
    fn escalation_promotes_a_droid_on_the_wave_timer() {
        let mut simulation = Simulation::new(11);
        start(&mut simulation);
        simulation.play_phase = PlayPhase::Active;
        simulation.wave_age = super::ESCALATION_FIRST_SECONDS - TICK_SECONDS * 0.5;
        simulation.update_escalation();
        assert!(simulation
            .enemies
            .iter()
            .any(|enemy| enemy.kind == EnemyKind::Command));
    }

    #[test]
    fn lone_survivor_fast_tracks_from_droid_to_death_ship() {
        let mut simulation = Simulation::new(12);
        start(&mut simulation);
        simulation.play_phase = PlayPhase::Active;
        simulation.enemies.truncate(1);
        simulation.last_ship_count = 1;
        simulation.next_escalation = f32::MAX;
        simulation.lone_timer = TICK_SECONDS * 0.5;
        simulation.update_escalation();
        assert_eq!(simulation.enemies[0].kind, EnemyKind::Command);
        simulation.last_ship_count = 1;
        simulation.lone_timer = TICK_SECONDS * 0.5;
        simulation.update_escalation();
        assert_eq!(simulation.enemies[0].kind, EnemyKind::Death);
    }

    #[test]
    fn global_mine_cap_is_respected() {
        let mut simulation = Simulation::new(13);
        start(&mut simulation);
        simulation.enemies.clear();
        for index in 0..MINE_CAP {
            let position = vec2(40.0 + index as f32 * 20.0, 100.0);
            assert!(simulation.try_spawn_mine(EnemyKind::PhotonMine, position, 0.0));
        }
        assert!(!simulation.try_spawn_mine(EnemyKind::VaporMine, vec2(50.0, 200.0), 0.0));
        assert_eq!(simulation.mine_count(), MINE_CAP);
    }

    #[test]
    fn every_fourth_wave_bonus_awards_points_and_sweeps_mines() {
        for wave in [4, 8, 12] {
            let mut simulation = Simulation::new(14);
            start(&mut simulation);
            simulation.wave = wave;
            simulation.enemies.clear();
            simulation
                .enemies
                .push(Enemy::mine(EnemyKind::PhotonMine, vec2(100.0, 100.0), 0.0));
            simulation.complete_wave();
            assert_eq!(simulation.score, super::FLEET_BONUS_POINTS);
            assert_eq!(simulation.mine_count(), 0);
            assert_eq!(simulation.play_phase, PlayPhase::FleetBonus);
        }
    }

    #[test]
    fn score_crossings_award_each_extra_ship_once() {
        let mut simulation = Simulation::new(15);
        start(&mut simulation);
        simulation.award_points(39_999);
        assert_eq!(simulation.ships, 3);
        simulation.award_points(1);
        assert_eq!(simulation.ships, 4);
        simulation.award_points(60_000);
        assert_eq!(simulation.ships, 5);
        simulation.award_points(100_000);
        assert_eq!(simulation.ships, 6);
        simulation.award_points(100_000);
        assert_eq!(simulation.ships, 7);
    }

    #[test]
    fn start_transitions_from_attract_to_playing() {
        let mut simulation = Simulation::new(16);
        assert_eq!(simulation.state, GameState::Attract);
        start(&mut simulation);
        assert_eq!(simulation.state, GameState::Playing);
    }

    #[test]
    fn pause_dims_the_scene_and_adds_a_bright_overlay() {
        let mut simulation = Simulation::new(17);
        start(&mut simulation);
        let running = simulation.display_list();
        simulation.tick(InputState {
            pause: true,
            ..InputState::default()
        });
        let paused = simulation.display_list();
        assert!(paused.segments.len() > running.segments.len());
        for (before, after) in running.segments.iter().zip(&paused.segments) {
            assert_eq!(before.a, after.a);
            assert_eq!(before.b, after.b);
            assert!((after.intensity - before.intensity * 0.5).abs() < 0.0001);
        }
        assert!(paused.segments[running.segments.len()..]
            .iter()
            .any(|segment| segment.intensity == 1.0));
    }

    #[test]
    fn escape_returns_play_to_attract_then_requests_quit() {
        let mut simulation = Simulation::new(18);
        start(&mut simulation);
        simulation.tick(InputState {
            escape: true,
            ..InputState::default()
        });
        assert_eq!(simulation.state, GameState::Attract);
        simulation.tick(InputState::default());
        simulation.tick(InputState {
            escape: true,
            ..InputState::default()
        });
        assert!(simulation.quit_requested());
    }
}
