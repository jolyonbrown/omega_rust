mod render;

use std::{
    f32::consts::{LN_2, PI, TAU},
    mem,
};

use macroquad::math::{vec2, Vec2};

use crate::{
    enemies::{circuit_pose, Enemy, EnemyBullet, EnemyKind, MINE_CAP},
    game::{
        is_fleet_bonus_wave, next_extra_ship_threshold, wave_size, Difficulty, GameState, PlayPhase,
    },
    hiscore::Storage as HighScoreStorage,
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
const SHRAPNEL_HALF_LENGTH: f32 = 7.5;
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
const LONE_PROMOTION_SECONDS: f32 = 3.0;
const FLEET_BONUS_POINTS: u32 = 5_000;
const DROID_BULLET_SPEED_SCALE: f32 = 0.85;
const MINE_CHAIN_FUSE_MIN_SECONDS: f32 = 0.12;
const MINE_CHAIN_FUSE_MAX_SECONDS: f32 = 0.30;
const MINE_BLAST_VISUAL_SECONDS: f32 = 0.4;
const SHRAPNEL_LIFETIME_SECONDS: f32 = 0.7;

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
    pub wave_select: bool,
    pub pause: bool,
    pub mute: bool,
    pub escape: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SfxEvent {
    Fire,
    Bounce,
    EnemyBounce,
    EnemyExplode,
    PlayerExplode,
    MinePop,
    MineArm,
    MineBlast,
    BulletFizzle,
    ExtraShip,
    WaveClear,
    FleetBonus,
    MineSweep,
    Promote,
    ThrustOn,
    ThrustOff,
    ConvoyTick { living_ships: u8, wave_ships: u8 },
}

impl InputState {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn union(self, other: Self) -> Self {
        Self {
            left: self.left || other.left,
            right: self.right || other.right,
            thrust: self.thrust || other.thrust,
            fire: self.fire || other.fire,
            start: self.start || other.start,
            wave_select: self.wave_select || other.wave_select,
            pause: self.pause || other.pause,
            mute: self.mute || other.mute,
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct MineBlastVisual {
    position: Vec2,
    radius: f32,
    age: f32,
}

#[derive(Clone, Debug)]
pub struct Simulation {
    pub player: Player,
    shots: Vec<Shot>,
    enemies: Vec<Enemy>,
    enemy_bullets: Vec<EnemyBullet>,
    particles: Vec<Particle>,
    mine_blasts: Vec<MineBlastVisual>,
    border_flash: [f32; 8],
    rng: Rng,
    seed: u64,
    frame: u64,
    state_frame: u64,
    previous_fire: bool,
    previous_start: bool,
    previous_wave_select: bool,
    previous_pause: bool,
    previous_escape: bool,
    paused: bool,
    quit_requested: bool,
    thrusting: bool,
    exhaust_length: f32,
    exhaust_spread: f32,
    state: GameState,
    play_phase: PlayPhase,
    difficulty: Difficulty,
    phase_timer: f32,
    death_timer: f32,
    game_over_timer: f32,
    wave_age: f32,
    next_escalation: f32,
    mine_drop_timer: f32,
    droid_fire_timer: f32,
    lone_timer: f32,
    last_ship_count: usize,
    convoy_direction: f32,
    score: u32,
    score_flash: f32,
    high_score: u32,
    ships: u32,
    wave: u32,
    practice_wave: u32,
    practice_run: bool,
    next_extra_ship: u32,
    extra_ship_flash: f32,
    new_high_score: bool,
    high_score_storage: HighScoreStorage,
    sfx_events: Vec<SfxEvent>,
    convoy_tick_timer: f32,
    convoy_wave_ships: usize,
}

impl Simulation {
    /// Deterministic, persistence-free simulation used by headless mode and tests.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(seed: u64) -> Self {
        Self::with_high_score(
            seed,
            crate::hiscore::DEFAULT_HIGH_SCORE,
            HighScoreStorage::session(),
        )
    }

    /// Windowed simulation. This is the only constructor that reads high-score data.
    pub fn persistent(seed: u64) -> Self {
        let high_score_storage = HighScoreStorage::persistent();
        let high_score = high_score_storage.load();
        Self::with_high_score(seed, high_score, high_score_storage)
    }

    fn with_high_score(seed: u64, high_score: u32, high_score_storage: HighScoreStorage) -> Self {
        let difficulty = Difficulty::for_wave(1);
        Self {
            player: spawn_player(),
            shots: Vec::with_capacity(MAX_SHOTS),
            enemies: Vec::new(),
            enemy_bullets: Vec::new(),
            particles: Vec::new(),
            mine_blasts: Vec::new(),
            border_flash: [0.0; 8],
            rng: Rng::new(seed),
            seed,
            frame: 0,
            state_frame: 0,
            previous_fire: false,
            previous_start: false,
            previous_wave_select: false,
            previous_pause: false,
            previous_escape: false,
            paused: false,
            quit_requested: false,
            thrusting: false,
            exhaust_length: 12.0,
            exhaust_spread: 3.0,
            state: GameState::Attract,
            play_phase: PlayPhase::Warning,
            difficulty,
            phase_timer: 0.0,
            death_timer: 0.0,
            game_over_timer: 0.0,
            wave_age: 0.0,
            next_escalation: difficulty.escalation_first_seconds,
            mine_drop_timer: 3.4,
            droid_fire_timer: 0.0,
            lone_timer: LONE_PROMOTION_SECONDS,
            last_ship_count: 0,
            convoy_direction: 1.0,
            score: 0,
            score_flash: 0.0,
            high_score,
            ships: 3,
            wave: 1,
            practice_wave: 1,
            practice_run: false,
            next_extra_ship: 40_000,
            extra_ship_flash: 0.0,
            new_high_score: false,
            high_score_storage,
            sfx_events: Vec::new(),
            convoy_tick_timer: 0.0,
            convoy_wave_ships: 0,
        }
    }

    pub fn tick(&mut self, input: InputState) {
        self.sfx_events.clear();
        let start_pressed = input.start && !self.previous_start;
        let wave_select_pressed = input.wave_select && !self.previous_wave_select;
        let pause_pressed = input.pause && !self.previous_pause;
        let escape_pressed = input.escape && !self.previous_escape;

        self.previous_start = input.start;
        self.previous_wave_select = input.wave_select;
        self.previous_pause = input.pause;
        self.previous_escape = input.escape;

        if escape_pressed {
            if self.state == GameState::Attract {
                self.quit_requested = true;
            } else {
                self.enter_attract();
            }
        }

        if wave_select_pressed && self.state == GameState::Attract {
            self.practice_wave = match self.practice_wave {
                1 => 14,
                14 => 17,
                17 => 21,
                21 => 25,
                _ => 1,
            };
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
            self.set_thrusting(false);
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
        self.update_mine_blast_visuals();
        self.update_convoy_tick();

        self.previous_fire = input.fire;
        self.frame = self.frame.wrapping_add(1);
        self.state_frame = self.state_frame.wrapping_add(1);
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    pub fn drain_sfx_events(&mut self) -> Vec<SfxEvent> {
        mem::take(&mut self.sfx_events)
    }

    #[cfg(test)]
    pub fn shot_count(&self) -> usize {
        self.shots.len()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_at_wave(seed: u64, wave: u32) -> Self {
        let mut simulation = Self::new(seed);
        simulation.practice_wave = wave.max(1);
        simulation.start_game();
        simulation
    }

    fn start_game(&mut self) {
        self.rng = Rng::new(self.seed);
        self.player = spawn_player();
        self.shots.clear();
        self.enemies.clear();
        self.enemy_bullets.clear();
        self.particles.clear();
        self.mine_blasts.clear();
        self.border_flash = [0.0; 8];
        self.state = GameState::Playing;
        self.state_frame = 0;
        self.paused = false;
        self.set_thrusting(false);
        self.score = 0;
        self.score_flash = 0.0;
        self.ships = 3;
        self.wave = self.practice_wave;
        self.practice_run = self.practice_wave != 1;
        self.next_extra_ship = 40_000;
        self.extra_ship_flash = 0.0;
        self.new_high_score = false;
        self.spawn_wave();
    }

    fn enter_attract(&mut self) {
        self.state = GameState::Attract;
        self.state_frame = 0;
        self.paused = false;
        self.set_thrusting(false);
        self.shots.clear();
        self.enemies.clear();
        self.enemy_bullets.clear();
        self.particles.clear();
        self.mine_blasts.clear();
    }

    fn enter_game_over(&mut self) {
        self.state = GameState::GameOver;
        self.state_frame = 0;
        self.game_over_timer = GAME_OVER_SECONDS;
        self.paused = false;
        self.set_thrusting(false);
        self.shots.clear();
        self.enemies.clear();
        self.enemy_bullets.clear();
        self.new_high_score = !self.practice_run && self.score > self.high_score;
        if self.new_high_score {
            self.high_score = self.score;
            self.high_score_storage.save(self.high_score);
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
                    self.next_escalation = self.difficulty.escalation_first_seconds;
                    self.mine_drop_timer = self.rng.range_f32(
                        self.difficulty.droid_initial_mine_min_seconds,
                        self.difficulty.droid_initial_mine_max_seconds,
                    );
                }
            }
            PlayPhase::Active => {
                self.update_player(input);
                self.update_shots_walls();
                self.update_enemies();
                self.update_enemy_bullets();
                self.update_escalation();
                self.update_droid_mine_drop();
                self.update_droid_fire();
                self.handle_combat();
                self.update_proximity_mines();
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
            self.clear_respawn_mines();
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
        self.set_thrusting(input.thrust);
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
        if resolve_circle_arena(
            &mut self.player.position,
            &mut self.player.velocity,
            SHIP_RADIUS,
            &mut self.border_flash,
            RESTITUTION,
        ) {
            self.sfx_events.push(SfxEvent::Bounce);
        }

        if input.fire && !self.previous_fire && self.shots.len() < MAX_SHOTS {
            self.shots.push(Shot {
                position: self.player.position + facing * 17.0,
                velocity: facing * SHOT_SPEED,
            });
            self.sfx_events.push(SfxEvent::Fire);
        }
    }

    fn update_shots_walls(&mut self) {
        let mut flashes = [false; 8];
        let mut fizzles = 0;
        self.shots.retain_mut(|shot| {
            let previous = shot.position;
            shot.position += shot.velocity * TICK_SECONDS;
            if let Some(edge) = projectile_collision_edge(previous, shot.position) {
                flashes[edge] = true;
                fizzles += 1;
                false
            } else {
                true
            }
        });
        apply_flashes(&mut self.border_flash, flashes);
        self.sfx_events
            .extend((0..fizzles).map(|_| SfxEvent::BulletFizzle));
    }

    fn update_enemy_bullets(&mut self) {
        let mut flashes = [false; 8];
        let mut fizzles = 0;
        self.enemy_bullets.retain_mut(|bullet| {
            let previous = bullet.position;
            bullet.position += bullet.velocity * TICK_SECONDS;
            if let Some(ttl) = &mut bullet.ttl {
                *ttl -= TICK_SECONDS;
                if *ttl <= 0.0 {
                    return false;
                }
            }
            if let Some(edge) = projectile_collision_edge(previous, bullet.position) {
                flashes[edge] = true;
                fizzles += 1;
                false
            } else {
                true
            }
        });
        apply_flashes(&mut self.border_flash, flashes);
        self.sfx_events
            .extend((0..fizzles).map(|_| SfxEvent::BulletFizzle));
    }

    fn update_enemies(&mut self) {
        let player_position = self.player.position;
        let convoy_direction = self.convoy_direction;
        let difficulty = self.difficulty;
        let mut new_bullets = Vec::new();
        let mut vapor_mines = Vec::new();
        let mut enemy_bounces = 0;

        for enemy in &mut self.enemies {
            enemy.age += TICK_SECONDS;
            match enemy.kind {
                EnemyKind::PhotonMine => {}
                EnemyKind::VaporMine => {
                    if difficulty.overdrive > 0.0 && enemy.velocity != Vec2::ZERO {
                        enemy.position += enemy.velocity * TICK_SECONDS;
                        resolve_circle_arena(
                            &mut enemy.position,
                            &mut enemy.velocity,
                            enemy.kind.radius(),
                            &mut self.border_flash,
                            1.0,
                        );
                    }
                }
                EnemyKind::Droid => {
                    enemy.path_distance +=
                        convoy_direction * difficulty.convoy_speed * TICK_SECONDS;
                    let (path_position, tangent, normal) = circuit_pose(enemy.path_distance);
                    let loose_jitter =
                        enemy.jitter + (enemy.age * 1.7 + enemy.wander_phase).sin() * 3.0;
                    let next_position = path_position + normal * loose_jitter;
                    enemy.velocity = (next_position - enemy.position) / TICK_SECONDS;
                    enemy.position = next_position;
                    if convoy_direction < 0.0 {
                        enemy.velocity = -tangent * difficulty.convoy_speed
                            + normal * (enemy.age * 1.7 + enemy.wander_phase).cos() * 5.1;
                    }
                    enemy.rotation = enemy.velocity.y.atan2(enemy.velocity.x);
                }
                EnemyKind::Command => {
                    let speed = enemy.velocity.length().clamp(
                        difficulty.command_wander_min_speed,
                        difficulty.command_wander_max_speed,
                    );
                    let steering = (enemy.age * 0.73 + enemy.wander_phase).sin() * 0.52;
                    let angle = enemy.velocity.y.atan2(enemy.velocity.x) + steering * TICK_SECONDS;
                    enemy.velocity = direction(angle) * speed;
                    enemy.position += enemy.velocity * TICK_SECONDS;
                    if resolve_circle_arena(
                        &mut enemy.position,
                        &mut enemy.velocity,
                        enemy.kind.radius(),
                        &mut self.border_flash,
                        RESTITUTION,
                    ) {
                        enemy_bounces += 1;
                    }
                    enemy.rotation = enemy.velocity.y.atan2(enemy.velocity.x);

                    enemy.action_timer -= TICK_SECONDS;
                    if enemy.action_timer <= 0.0 {
                        let aim_error = self.rng.range_f32(
                            -difficulty.command_aim_error_radians,
                            difficulty.command_aim_error_radians,
                        );
                        let aim = (player_position - enemy.position).normalize_or_zero();
                        let angle = aim.y.atan2(aim.x) + aim_error;
                        new_bullets.push(EnemyBullet {
                            position: enemy.position + direction(angle) * 15.0,
                            velocity: direction(angle) * difficulty.enemy_bullet_speed,
                            ttl: None,
                        });
                        enemy.action_timer = self.rng.range_f32(
                            difficulty.command_fire_min_seconds,
                            difficulty.command_fire_max_seconds,
                        );
                    }

                    enemy.mine_timer -= TICK_SECONDS;
                    if enemy.mine_timer <= 0.0 {
                        vapor_mines.push((
                            enemy.position - enemy.velocity.normalize_or_zero() * 18.0,
                            enemy.rotation,
                        ));
                        enemy.mine_timer = self.rng.range_f32(
                            difficulty.command_mine_min_seconds,
                            difficulty.command_mine_max_seconds,
                        );
                    }
                }
                EnemyKind::Death => {
                    let desired = (player_position - enemy.position).normalize_or_zero();
                    enemy.velocity +=
                        desired * difficulty.death_steering_acceleration * TICK_SECONDS;
                    enemy.velocity = enemy.velocity.clamp_length_max(difficulty.death_max_speed);
                    enemy.position += enemy.velocity * TICK_SECONDS;
                    if resolve_circle_arena(
                        &mut enemy.position,
                        &mut enemy.velocity,
                        enemy.kind.radius(),
                        &mut self.border_flash,
                        RESTITUTION,
                    ) {
                        enemy_bounces += 1;
                    }
                    enemy.rotation = enemy.velocity.y.atan2(enemy.velocity.x);
                }
            }
        }
        self.sfx_events
            .extend((0..enemy_bounces).map(|_| SfxEvent::EnemyBounce));
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
            self.next_escalation += self.difficulty.escalation_repeat_seconds;
        }

        let aged_commands: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| {
                (enemy.kind == EnemyKind::Command
                    && enemy.age >= self.difficulty.command_promotion_seconds)
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
        let wander_speed = (self.difficulty.command_wander_min_speed
            + self.difficulty.command_wander_max_speed)
            * 0.5;
        enemy.velocity = enemy.velocity.normalize_or_zero() * wander_speed;
        enemy.action_timer = self.rng.range_f32(
            self.difficulty.command_fire_min_seconds,
            self.difficulty.command_fire_max_seconds,
        );
        enemy.mine_timer = self.rng.range_f32(
            self.difficulty.command_initial_mine_min_seconds,
            self.difficulty.command_initial_mine_max_seconds,
        );
        enemy.wander_phase = self.rng.range_f32(0.0, TAU);
        self.sfx_events.push(SfxEvent::Promote);
    }

    fn promote_to_death(&mut self, index: usize) {
        let enemy = &mut self.enemies[index];
        if enemy.kind != EnemyKind::Command {
            return;
        }
        enemy.kind = EnemyKind::Death;
        enemy.age = 0.0;
        enemy.velocity = enemy.velocity.normalize_or_zero() * 220.0;
        self.sfx_events.push(SfxEvent::Promote);
    }

    fn update_droid_mine_drop(&mut self) {
        self.mine_drop_timer -= TICK_SECONDS;
        if self.mine_drop_timer > 0.0 {
            return;
        }
        self.mine_drop_timer = self.rng.range_f32(
            self.difficulty.droid_mine_min_seconds,
            self.difficulty.droid_mine_max_seconds,
        );
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

    fn update_droid_fire(&mut self) {
        if self.difficulty.overdrive <= 0.0
            || self.state != GameState::Playing
            || self.play_phase != PlayPhase::Active
        {
            return;
        }
        let droids: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(index, enemy)| (enemy.kind == EnemyKind::Droid).then_some(index))
            .collect();
        if droids.is_empty() {
            return;
        }

        self.droid_fire_timer -= TICK_SECONDS;
        if self.droid_fire_timer > 0.0 {
            return;
        }

        let choice = self.rng.next_u64() as usize % droids.len();
        let droid = &self.enemies[droids[choice]];
        let aim_error = self.rng.range_f32(
            -overdrive_lerp(
                16.0_f32.to_radians(),
                9.0_f32.to_radians(),
                self.difficulty.overdrive,
            ),
            overdrive_lerp(
                16.0_f32.to_radians(),
                9.0_f32.to_radians(),
                self.difficulty.overdrive,
            ),
        );
        let aim = (self.player.position - droid.position).normalize_or_zero();
        let angle = aim.y.atan2(aim.x) + aim_error;
        self.enemy_bullets.push(EnemyBullet {
            position: droid.position + direction(angle) * 15.0,
            velocity: direction(angle)
                * self.difficulty.enemy_bullet_speed
                * DROID_BULLET_SPEED_SCALE,
            ttl: None,
        });
        self.droid_fire_timer = self.random_droid_fire_interval();
    }

    fn random_droid_fire_interval(&mut self) -> f32 {
        self.rng.range_f32(
            overdrive_lerp(3.6, 1.5, self.difficulty.overdrive),
            overdrive_lerp(5.4, 2.6, self.difficulty.overdrive),
        )
    }

    fn try_spawn_mine(&mut self, kind: EnemyKind, position: Vec2, rotation: f32) -> bool {
        if self.mine_count() >= MINE_CAP
            || position.distance(self.player.position) < RESPAWN_CLEARANCE - 20.0
            || !circle_fits_track(position, kind.radius())
        {
            return false;
        }
        let mut mine = Enemy::mine(kind, position, rotation);
        if kind == EnemyKind::VaporMine && self.difficulty.overdrive > 0.0 {
            let heading = self.rng.range_f32(0.0, TAU);
            let speed = overdrive_lerp(12.0, 40.0, self.difficulty.overdrive);
            mine.velocity = direction(heading) * speed;
        }
        self.enemies.push(mine);
        true
    }

    fn update_proximity_mines(&mut self) {
        if self.difficulty.overdrive <= 0.0
            || self.state != GameState::Playing
            || self.play_phase != PlayPhase::Active
        {
            return;
        }

        let trigger_radius = overdrive_lerp(70.0, 100.0, self.difficulty.overdrive);
        let fuse_seconds = overdrive_lerp(0.9, 0.65, self.difficulty.overdrive);
        for enemy in &mut self.enemies {
            if enemy.kind == EnemyKind::VaporMine
                && !enemy.armed
                && enemy.position.distance(self.player.position) <= trigger_radius
            {
                enemy.armed = true;
                enemy.armed_age = 0.0;
                enemy.action_timer = fuse_seconds;
                self.sfx_events.push(SfxEvent::MineArm);
            }
        }

        for enemy in &mut self.enemies {
            if enemy.kind == EnemyKind::VaporMine && enemy.armed {
                enemy.armed_age += TICK_SECONDS;
                enemy.action_timer -= TICK_SECONDS;
            }
        }

        let old_enemies = mem::take(&mut self.enemies);
        let mut detonations = Vec::new();
        for enemy in old_enemies {
            if enemy.kind == EnemyKind::VaporMine && enemy.armed && enemy.action_timer <= 0.0 {
                detonations.push(enemy);
            } else {
                self.enemies.push(enemy);
            }
        }
        for mine in detonations {
            self.detonate_vapor_mine(mine);
        }
    }

    fn detonate_vapor_mine(&mut self, mine: Enemy) {
        let radius = overdrive_lerp(60.0, 110.0, self.difficulty.overdrive);
        self.sfx_events.push(SfxEvent::MineBlast);
        self.mine_blasts.push(MineBlastVisual {
            position: mine.position,
            radius,
            age: 0.0,
        });
        self.spawn_enemy_shatter(&mine, 0.48, 190.0);
        self.spawn_shrapnel(mine.position);
        self.apply_mine_blast(mine.position, radius);
    }

    fn apply_mine_blast(&mut self, center: Vec2, radius: f32) {
        let player_hit = self.player.position.distance(center) <= radius + SHIP_RADIUS;
        let old_enemies = mem::take(&mut self.enemies);
        for mut enemy in old_enemies {
            if enemy.position.distance(center) > radius + enemy.kind.radius() {
                self.enemies.push(enemy);
                continue;
            }
            match enemy.kind {
                EnemyKind::PhotonMine => {
                    self.sfx_events.push(SfxEvent::MinePop);
                    self.spawn_enemy_shatter(&enemy, 0.9, 115.0);
                    self.award_points(enemy.kind.points());
                }
                EnemyKind::VaporMine => {
                    let newly_armed = !enemy.armed;
                    enemy.armed = true;
                    enemy.armed_age = 0.0;
                    enemy.action_timer = self
                        .rng
                        .range_f32(MINE_CHAIN_FUSE_MIN_SECONDS, MINE_CHAIN_FUSE_MAX_SECONDS);
                    if newly_armed {
                        self.sfx_events.push(SfxEvent::MineArm);
                    }
                    self.enemies.push(enemy);
                }
                EnemyKind::Droid | EnemyKind::Command | EnemyKind::Death => {
                    self.sfx_events.push(SfxEvent::EnemyExplode);
                    self.spawn_enemy_shatter(&enemy, 0.9, 115.0);
                    self.award_points(enemy.kind.points());
                }
            }
        }
        if player_hit {
            self.kill_player();
        }
    }

    fn spawn_shrapnel(&mut self, center: Vec2) {
        let overdrive = self.difficulty.overdrive;
        if overdrive < 0.3 {
            return;
        }
        let progress = ((overdrive - 0.3) / 0.7).clamp(0.0, 1.0);
        let count = overdrive_lerp(3.0, 6.0, progress).round() as usize;
        let offset = self.rng.range_f32(0.0, TAU);
        for index in 0..count {
            let angle = offset + TAU * index as f32 / count as f32;
            self.enemy_bullets.push(EnemyBullet {
                position: center,
                velocity: direction(angle) * self.rng.range_f32(230.0, 300.0),
                ttl: Some(SHRAPNEL_LIFETIME_SECONDS),
            });
        }
    }

    fn update_mine_blast_visuals(&mut self) {
        for blast in &mut self.mine_blasts {
            blast.age += TICK_SECONDS;
        }
        self.mine_blasts
            .retain(|blast| blast.age < MINE_BLAST_VISUAL_SECONDS);
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
                self.sfx_events.push(if enemy.kind.is_mine() {
                    SfxEvent::MinePop
                } else {
                    SfxEvent::EnemyExplode
                });
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
        self.sfx_events.push(SfxEvent::PlayerExplode);
        self.state = GameState::ShipDeath;
        self.state_frame = 0;
        self.death_timer = SHIP_DEATH_SECONDS;
        self.set_thrusting(false);
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

    fn clear_respawn_mines(&mut self) {
        let spawn = spawn_position();
        let old_enemies = mem::take(&mut self.enemies);
        for enemy in old_enemies {
            if enemy.kind.is_mine() && enemy.position.distance(spawn) < RESPAWN_CLEARANCE {
                self.spawn_enemy_shatter(&enemy, 0.35, 28.0);
            } else {
                self.enemies.push(enemy);
            }
        }
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
            self.sfx_events.push(SfxEvent::FleetBonus);
            self.sfx_events.push(SfxEvent::MineSweep);
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
            self.sfx_events.push(SfxEvent::WaveClear);
            self.play_phase = PlayPhase::WaveCleared;
            self.phase_timer = WAVE_CLEARED_SECONDS;
        }
    }

    fn award_points(&mut self, points: u32) {
        self.score = self.score.saturating_add(points);
        self.score_flash = SCORE_FLASH_SECONDS;
        while self.next_extra_ship != u32::MAX && self.score >= self.next_extra_ship {
            self.ships = self.ships.saturating_add(1);
            self.sfx_events.push(SfxEvent::ExtraShip);
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
        self.difficulty = Difficulty::for_wave(self.wave);
        self.enemies.retain(|enemy| enemy.kind.is_mine());
        self.enemy_bullets.clear();
        self.shots.clear();
        self.convoy_direction = if self.rng.next_f32() < 0.5 { -1.0 } else { 1.0 };
        let count = wave_size(self.wave);
        for index in 0..count {
            let distance = 72.0 + index as f32 * 58.0;
            let jitter = self.rng.range_f32(-9.0, 9.0);
            self.enemies.push(Enemy::droid(
                distance,
                jitter,
                self.convoy_direction,
                self.difficulty.convoy_speed,
            ));
        }
        self.play_phase = PlayPhase::Warning;
        self.phase_timer = SPAWN_WARNING_SECONDS;
        self.wave_age = 0.0;
        self.next_escalation = self.difficulty.escalation_first_seconds;
        self.mine_drop_timer = 3.4;
        if self.difficulty.overdrive > 0.0 {
            self.droid_fire_timer = self.random_droid_fire_interval() + 2.0;
        }
        self.lone_timer = LONE_PROMOTION_SECONDS;
        self.last_ship_count = count;
        self.convoy_tick_timer = 0.0;
        self.convoy_wave_ships = count;
    }

    fn set_thrusting(&mut self, thrusting: bool) {
        if self.thrusting == thrusting {
            return;
        }
        self.thrusting = thrusting;
        self.sfx_events.push(if thrusting {
            SfxEvent::ThrustOn
        } else {
            SfxEvent::ThrustOff
        });
    }

    fn update_convoy_tick(&mut self) {
        if !matches!(self.state, GameState::Playing | GameState::ShipDeath)
            || self.play_phase != PlayPhase::Active
        {
            return;
        }
        let living_ships = self.ship_count();
        if living_ships == 0 || self.convoy_wave_ships == 0 {
            return;
        }

        self.convoy_tick_timer -= TICK_SECONDS;
        if self.convoy_tick_timer > 0.0 {
            return;
        }
        self.sfx_events.push(SfxEvent::ConvoyTick {
            living_ships: living_ships.min(u8::MAX as usize) as u8,
            wave_ships: self.convoy_wave_ships.min(u8::MAX as usize) as u8,
        });

        let eliminated = self.convoy_wave_ships.saturating_sub(living_ships) as f32;
        let denominator = self.convoy_wave_ships.saturating_sub(1).max(1) as f32;
        let pressure = (eliminated / denominator).clamp(0.0, 1.0);
        let interval = 0.72 + (0.22 - 0.72) * pressure;
        self.convoy_tick_timer += interval;
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
    velocity - (1.0 + restitution) * velocity.dot(normal) * normal
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

fn overdrive_lerp(start: f32, end: f32, overdrive: f32) -> f32 {
    if overdrive <= 0.0 {
        start
    } else if overdrive >= 1.0 {
        end
    } else {
        start + (end - start) * overdrive
    }
}

fn resolve_circle_arena(
    position: &mut Vec2,
    velocity: &mut Vec2,
    radius: f32,
    border_flash: &mut [f32; 8],
    restitution: f32,
) -> bool {
    let mut bounced = false;
    if resolve_axis_wall(
        &mut position.x,
        OUTER_LEFT + radius,
        true,
        velocity,
        vec2(1.0, 0.0),
        restitution,
    ) {
        flash_border_edge(border_flash, OUTER_LEFT_EDGE);
        bounced = true;
    }
    if resolve_axis_wall(
        &mut position.x,
        OUTER_RIGHT - radius,
        false,
        velocity,
        vec2(-1.0, 0.0),
        restitution,
    ) {
        flash_border_edge(border_flash, OUTER_RIGHT_EDGE);
        bounced = true;
    }
    if resolve_axis_wall(
        &mut position.y,
        OUTER_TOP + radius,
        true,
        velocity,
        vec2(0.0, 1.0),
        restitution,
    ) {
        flash_border_edge(border_flash, OUTER_TOP_EDGE);
        bounced = true;
    }
    if resolve_axis_wall(
        &mut position.y,
        OUTER_BOTTOM - radius,
        false,
        velocity,
        vec2(0.0, -1.0),
        restitution,
    ) {
        flash_border_edge(border_flash, OUTER_BOTTOM_EDGE);
        bounced = true;
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
            *velocity = reflect_velocity(*velocity, normal, restitution);
            flash_border_edge(border_flash, edge);
            bounced = true;
        }
    }
    bounced
}

fn resolve_axis_wall(
    coordinate: &mut f32,
    limit: f32,
    is_minimum: bool,
    velocity: &mut Vec2,
    inward_normal: Vec2,
    restitution: f32,
) -> bool {
    let crossed = if is_minimum {
        *coordinate < limit
    } else {
        *coordinate > limit
    };
    if crossed {
        *coordinate = limit;
        if velocity.dot(inward_normal) < 0.0 {
            *velocity = reflect_velocity(*velocity, inward_normal, restitution);
            return true;
        }
    }
    false
}

fn projectile_collision_edge(previous: Vec2, position: Vec2) -> Option<usize> {
    let walls = [
        (
            vec2(OUTER_LEFT, OUTER_TOP),
            vec2(OUTER_RIGHT, OUTER_TOP),
            OUTER_TOP_EDGE,
        ),
        (
            vec2(OUTER_RIGHT, OUTER_TOP),
            vec2(OUTER_RIGHT, OUTER_BOTTOM),
            OUTER_RIGHT_EDGE,
        ),
        (
            vec2(OUTER_RIGHT, OUTER_BOTTOM),
            vec2(OUTER_LEFT, OUTER_BOTTOM),
            OUTER_BOTTOM_EDGE,
        ),
        (
            vec2(OUTER_LEFT, OUTER_BOTTOM),
            vec2(OUTER_LEFT, OUTER_TOP),
            OUTER_LEFT_EDGE,
        ),
        (
            vec2(CONSOLE_LEFT, CONSOLE_TOP),
            vec2(CONSOLE_RIGHT, CONSOLE_TOP),
            CONSOLE_TOP_EDGE,
        ),
        (
            vec2(CONSOLE_RIGHT, CONSOLE_TOP),
            vec2(CONSOLE_RIGHT, CONSOLE_BOTTOM),
            CONSOLE_RIGHT_EDGE,
        ),
        (
            vec2(CONSOLE_RIGHT, CONSOLE_BOTTOM),
            vec2(CONSOLE_LEFT, CONSOLE_BOTTOM),
            CONSOLE_BOTTOM_EDGE,
        ),
        (
            vec2(CONSOLE_LEFT, CONSOLE_BOTTOM),
            vec2(CONSOLE_LEFT, CONSOLE_TOP),
            CONSOLE_LEFT_EDGE,
        ),
    ];
    if let Some((_, edge)) = walls
        .into_iter()
        .filter_map(|(a, b, edge)| {
            segment_intersection_fraction(previous, position, a, b).map(|time| (time, edge))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
    {
        return Some(edge);
    }

    point_collision_edge(position)
}

fn point_collision_edge(position: Vec2) -> Option<usize> {
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

fn segment_intersection_fraction(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<f32> {
    let segment = b - a;
    let wall = d - c;
    let denominator = cross(segment, wall);
    if denominator.abs() <= f32::EPSILON {
        return None;
    }
    let offset = c - a;
    let time = cross(offset, wall) / denominator;
    let wall_time = cross(offset, segment) / denominator;
    ((0.0..=1.0).contains(&time) && (0.0..=1.0).contains(&wall_time)).then_some(time)
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn circle_fits_track(position: Vec2, radius: f32) -> bool {
    let inside_outer = position.x - radius >= OUTER_LEFT
        && position.x + radius <= OUTER_RIGHT
        && position.y - radius >= OUTER_TOP
        && position.y + radius <= OUTER_BOTTOM;
    let outside_console = position.x + radius <= CONSOLE_LEFT
        || position.x - radius >= CONSOLE_RIGHT
        || position.y + radius <= CONSOLE_TOP
        || position.y - radius >= CONSOLE_BOTTOM;
    inside_outer && outside_console
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
        circle_fits_track, reflect_velocity, Enemy, EnemyBullet, EnemyKind, GameState, InputState,
        PlayPhase, SfxEvent, Shot, Simulation, TICK_SECONDS,
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
    fn reflection_preserves_tangential_speed_and_damps_only_normal_speed() {
        let velocity = vec2(-30.0, 40.0);
        let normal = vec2(1.0, 0.0);
        let tangent = vec2(0.0, 1.0);
        let restitution = 0.9;
        let reflected = reflect_velocity(velocity, normal, restitution);
        assert!((reflected.dot(tangent) - velocity.dot(tangent)).abs() < 0.0001);
        assert!((reflected.dot(normal) + velocity.dot(normal) * restitution).abs() < 0.0001);
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
    fn shot_sweeping_through_a_console_corner_dies_at_the_wall() {
        let mut simulation = Simulation::new(21);
        let position = vec2(247.0, 292.0);
        let velocity = vec2(720.0, -480.0);
        let destination = position + velocity * TICK_SECONDS;
        assert_eq!(super::point_collision_edge(position), None);
        assert_eq!(super::point_collision_edge(destination), None);
        simulation.shots.push(Shot { position, velocity });

        simulation.update_shots_walls();

        assert!(simulation.shots.is_empty());
        assert!(simulation.border_flash[super::CONSOLE_LEFT_EDGE] > 0.0);
    }

    #[test]
    fn fire_and_thrust_events_are_tick_scoped_and_edge_triggered() {
        let mut simulation = Simulation::new(19);
        start(&mut simulation);
        simulation.tick(InputState {
            thrust: true,
            fire: true,
            ..InputState::default()
        });
        let events = simulation.drain_sfx_events();
        assert!(events.contains(&SfxEvent::ThrustOn));
        assert!(events.contains(&SfxEvent::Fire));

        simulation.tick(InputState {
            thrust: true,
            fire: true,
            ..InputState::default()
        });
        assert!(simulation.drain_sfx_events().is_empty());

        simulation.tick(InputState::default());
        assert_eq!(simulation.drain_sfx_events(), vec![SfxEvent::ThrustOff]);
    }

    #[test]
    fn convoy_ticks_accelerate_and_report_the_living_fleet() {
        let mut simulation = Simulation::new(20);
        start(&mut simulation);
        simulation.play_phase = PlayPhase::Active;
        simulation.convoy_tick_timer = 0.0;
        simulation.update_convoy_tick();
        let full_fleet_interval = simulation.convoy_tick_timer;
        assert!(simulation
            .drain_sfx_events()
            .contains(&SfxEvent::ConvoyTick {
                living_ships: 5,
                wave_ships: 5,
            }));

        simulation.enemies.truncate(1);
        simulation.convoy_tick_timer = 0.0;
        simulation.update_convoy_tick();
        let last_ship_interval = simulation.convoy_tick_timer;
        assert!(last_ship_interval < full_fleet_interval * 0.5);
        assert!(simulation
            .drain_sfx_events()
            .contains(&SfxEvent::ConvoyTick {
                living_ships: 1,
                wave_ships: 5,
            }));
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
        simulation.wave_age = simulation.difficulty.escalation_first_seconds - TICK_SECONDS * 0.5;
        simulation.update_escalation();
        assert!(simulation
            .enemies
            .iter()
            .any(|enemy| enemy.kind == EnemyKind::Command));
    }

    #[test]
    fn wave_thirteen_simulation_consumes_full_heat_tuning() {
        let mut simulation = Simulation::new_at_wave(25, 13);

        assert_eq!(simulation.difficulty.heat, 1.0);
        assert_eq!(simulation.difficulty.convoy_speed, 205.0);
        assert_eq!(simulation.difficulty.command_fire_min_seconds, 0.9);
        assert_eq!(simulation.difficulty.command_fire_max_seconds, 1.6);
        assert!(simulation.difficulty.death_max_speed < super::MAX_SPEED);
        assert_eq!(simulation.next_escalation, 6.0);
        assert!((simulation.enemies[0].velocity.length() - 205.0).abs() < 0.0001);

        simulation.promote_to_command(0);
        assert!((0.9..=1.6).contains(&simulation.enemies[0].action_timer));
    }

    #[test]
    fn wave_thirteen_keeps_every_overdrive_mechanic_and_rng_draw_inert() {
        let mut simulation = Simulation::new_at_wave(0x1313, 13);
        simulation.play_phase = PlayPhase::Active;
        simulation.next_escalation = f32::MAX;
        simulation.mine_drop_timer = f32::MAX;
        simulation.player.position = vec2(512.0, 560.0);
        simulation
            .enemies
            .push(Enemy::mine(EnemyKind::VaporMine, vec2(512.0, 540.0), 0.0));
        let rng_before = simulation.rng;

        for _ in 0..300 {
            simulation.tick(InputState::default());
            let events = simulation.drain_sfx_events();
            assert!(!events.contains(&SfxEvent::MineArm));
            assert!(!events.contains(&SfxEvent::MineBlast));
            assert_eq!(simulation.state, GameState::Playing);
        }

        let vapor = simulation
            .enemies
            .iter()
            .find(|enemy| enemy.kind == EnemyKind::VaporMine)
            .expect("the inert vapor mine persists");
        assert!(!vapor.armed);
        assert_eq!(vapor.velocity, vec2(0.0, 0.0));
        assert!(simulation.enemy_bullets.is_empty());
        assert!(simulation.mine_blasts.is_empty());
        assert_eq!(simulation.rng, rng_before);
    }

    #[test]
    fn overdrive_vapor_mine_arms_blasts_chains_and_scores_ship_kills() {
        let mut simulation = Simulation::new_at_wave(0x1414, 14);
        simulation.play_phase = PlayPhase::Active;
        simulation.next_escalation = f32::MAX;
        simulation.mine_drop_timer = f32::MAX;
        simulation.droid_fire_timer = f32::MAX;
        simulation.enemies.clear();
        simulation.player.position = vec2(190.0, 200.0);
        simulation
            .enemies
            .push(Enemy::mine(EnemyKind::VaporMine, vec2(120.0, 200.0), 0.0));
        simulation
            .enemies
            .push(Enemy::mine(EnemyKind::VaporMine, vec2(60.0, 200.0), 0.0));
        let mut command = Enemy::droid(0.0, 0.0, 1.0, 205.0);
        command.kind = EnemyKind::Command;
        command.position = vec2(120.0, 250.0);
        command.velocity = vec2(190.0, 0.0);
        command.action_timer = f32::MAX;
        command.mine_timer = f32::MAX;
        simulation.enemies.push(command);

        simulation.tick(InputState::default());
        assert!(simulation.drain_sfx_events().contains(&SfxEvent::MineArm));
        let source = simulation
            .enemies
            .iter_mut()
            .find(|enemy| {
                enemy.kind == EnemyKind::VaporMine
                    && enemy.position.distance(vec2(120.0, 200.0)) < 1.0
            })
            .expect("the proximity mine armed");
        assert!(source.armed);
        source.action_timer = 0.0;

        simulation.tick(InputState::default());
        let events = simulation.drain_sfx_events();
        assert!(events.contains(&SfxEvent::MineBlast));
        assert!(events.contains(&SfxEvent::MineArm));
        assert_eq!(simulation.state, GameState::ShipDeath);
        assert_eq!(simulation.score, EnemyKind::Command.points());
        assert!(!simulation.enemies.iter().any(|enemy| enemy.kind.is_ship()));
        let chained = simulation
            .enemies
            .iter()
            .find(|enemy| enemy.kind == EnemyKind::VaporMine)
            .expect("the nearby vapor mine survives and chain-arms");
        assert!(chained.armed);
        assert!(
            (super::MINE_CHAIN_FUSE_MIN_SECONDS..=super::MINE_CHAIN_FUSE_MAX_SECONDS)
                .contains(&chained.action_timer)
        );
    }

    #[test]
    fn armed_vapor_mines_defuse_normally_and_freeze_during_ship_death() {
        let mut defuse = Simulation::new_at_wave(0xDEF0, 25);
        defuse.play_phase = PlayPhase::Active;
        defuse.enemies.clear();
        let mut mine = Enemy::mine(EnemyKind::VaporMine, vec2(100.0, 100.0), 0.0);
        mine.armed = true;
        mine.action_timer = 0.5;
        defuse.enemies.push(mine);
        defuse.shots.push(Shot {
            position: vec2(105.0, 100.0),
            velocity: vec2(600.0, 0.0),
        });
        defuse.handle_combat();
        assert_eq!(defuse.score, EnemyKind::VaporMine.points());
        assert!(defuse.enemies.is_empty());
        let events = defuse.drain_sfx_events();
        assert!(events.contains(&SfxEvent::MinePop));
        assert!(!events.contains(&SfxEvent::MineBlast));

        let mut frozen = Simulation::new_at_wave(0xF0EE, 25);
        frozen.state = GameState::ShipDeath;
        frozen.play_phase = PlayPhase::Active;
        frozen.death_timer = 10.0;
        frozen.mine_drop_timer = f32::MAX;
        frozen.next_escalation = f32::MAX;
        frozen.enemies.clear();
        let mut armed = Enemy::mine(EnemyKind::VaporMine, vec2(100.0, 100.0), 0.0);
        armed.armed = true;
        armed.action_timer = 0.5;
        frozen.enemies.push(armed);
        frozen
            .enemies
            .push(Enemy::droid(0.0, 0.0, 1.0, frozen.difficulty.convoy_speed));
        frozen.tick(InputState::default());
        let armed = frozen
            .enemies
            .iter()
            .find(|enemy| enemy.kind == EnemyKind::VaporMine)
            .expect("the armed mine persists while the player is dead");
        assert_eq!(armed.action_timer, 0.5);
        assert_eq!(armed.armed_age, 0.0);
    }

    #[test]
    fn overdrive_vapor_mine_drift_reflects_and_stays_in_the_track() {
        let mut simulation = Simulation::new_at_wave(0x2525, 25);
        simulation.enemies.clear();
        assert!(simulation.try_spawn_mine(EnemyKind::VaporMine, vec2(100.0, 100.0), 0.0,));
        assert!(simulation.enemies[0].velocity.length() > 0.0);

        for _ in 0..5_000 {
            simulation.update_enemies();
            let mine = &simulation.enemies[0];
            assert!(circle_fits_track(mine.position, mine.kind.radius()));
        }
    }

    #[test]
    fn overdrive_shrapnel_has_the_scaled_count_ttl_and_wall_fizzle() {
        for (wave, expected) in [(17, 3), (25, 6)] {
            let mut simulation = Simulation::new_at_wave(0x5100 + wave as u64, wave);
            simulation.enemy_bullets.clear();
            simulation.spawn_shrapnel(vec2(100.0, 100.0));
            assert_eq!(simulation.enemy_bullets.len(), expected);
            assert!(simulation
                .enemy_bullets
                .iter()
                .all(|bullet| bullet.ttl == Some(super::SHRAPNEL_LIFETIME_SECONDS)));
            for _ in 0..60 {
                simulation.update_enemy_bullets();
            }
            assert!(simulation.enemy_bullets.is_empty());
        }

        let mut simulation = Simulation::new_at_wave(0x5151, 25);
        simulation.enemy_bullets.clear();
        simulation.enemy_bullets.push(EnemyBullet {
            position: vec2(super::OUTER_LEFT + 1.0, 100.0),
            velocity: vec2(-230.0, 0.0),
            ttl: Some(super::SHRAPNEL_LIFETIME_SECONDS),
        });
        simulation.update_enemy_bullets();
        assert!(simulation.enemy_bullets.is_empty());
        assert!(simulation
            .drain_sfx_events()
            .contains(&SfxEvent::BulletFizzle));
    }

    #[test]
    fn droids_only_return_fire_in_overdrive() {
        let mut wave_thirteen = Simulation::new_at_wave(0xD013, 13);
        wave_thirteen.play_phase = PlayPhase::Active;
        wave_thirteen.droid_fire_timer = 0.0;
        let rng_before = wave_thirteen.rng;
        wave_thirteen.update_droid_fire();
        assert!(wave_thirteen.enemy_bullets.is_empty());
        assert_eq!(wave_thirteen.rng, rng_before);

        let mut wave_twenty_five = Simulation::new_at_wave(0xD025, 25);
        wave_twenty_five.play_phase = PlayPhase::Active;
        wave_twenty_five.droid_fire_timer = 0.0;
        wave_twenty_five.update_droid_fire();
        assert_eq!(wave_twenty_five.enemy_bullets.len(), 1);
        let bullet = wave_twenty_five.enemy_bullets[0];
        assert_eq!(bullet.ttl, None);
        assert!(
            (bullet.velocity.length()
                - wave_twenty_five.difficulty.enemy_bullet_speed * super::DROID_BULLET_SPEED_SCALE)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn practice_runs_never_replace_the_stored_high_score() {
        let mut simulation = Simulation::new(0xCAFE);
        let original_high_score = simulation.high_score;
        simulation.practice_wave = 25;
        simulation.start_game();
        assert!(simulation.practice_run);
        simulation.score = original_high_score + 10_000;
        simulation.enter_game_over();
        assert_eq!(simulation.high_score, original_high_score);
        assert!(!simulation.new_high_score);

        simulation.enter_attract();
        simulation.practice_wave = 1;
        simulation.start_game();
        assert!(!simulation.practice_run);
        simulation.score = original_high_score + 1;
        simulation.enter_game_over();
        assert_eq!(simulation.high_score, original_high_score + 1);
        assert!(simulation.new_high_score);
    }

    #[test]
    fn attract_wave_select_cycles_and_start_honours_the_selection() {
        let mut simulation = Simulation::new(0xBEEF);
        for expected in [14, 17, 21, 25, 1] {
            simulation.tick(InputState {
                wave_select: true,
                ..InputState::default()
            });
            assert_eq!(simulation.practice_wave, expected);
            simulation.tick(InputState::default());
        }
        simulation.tick(InputState {
            wave_select: true,
            ..InputState::default()
        });
        simulation.tick(InputState::default());
        simulation.tick(InputState {
            start: true,
            ..InputState::default()
        });
        assert_eq!(simulation.wave, 14);
        assert!(simulation.practice_run);
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
    fn mine_spawn_requires_the_full_circle_to_fit_inside_the_track() {
        let mut simulation = Simulation::new(22);
        start(&mut simulation);
        simulation.enemies.clear();
        let radius = EnemyKind::VaporMine.radius();

        assert!(!simulation.try_spawn_mine(
            EnemyKind::VaporMine,
            vec2(super::OUTER_LEFT + radius - 0.1, 200.0),
            0.0,
        ));
        assert!(!simulation.try_spawn_mine(
            EnemyKind::VaporMine,
            vec2(super::CONSOLE_LEFT - radius + 0.1, 408.0),
            0.0,
        ));
        assert!(simulation.try_spawn_mine(
            EnemyKind::VaporMine,
            vec2(super::CONSOLE_LEFT - radius, 408.0),
            0.0,
        ));
    }

    #[test]
    fn respawn_sweeps_mines_from_the_clearance_zone() {
        let mut simulation = Simulation::new(23);
        start(&mut simulation);
        simulation.enemies.clear();
        let spawn = super::spawn_position();
        simulation
            .enemies
            .push(Enemy::mine(EnemyKind::PhotonMine, spawn, 0.0));

        simulation.tick(InputState::default());
        assert_eq!(simulation.state, GameState::ShipDeath);
        assert_eq!(simulation.ships, 2);

        for _ in 0..100 {
            simulation.tick(InputState::default());
            assert!(!simulation.drain_sfx_events().contains(&SfxEvent::MinePop));
            if simulation.state == GameState::Playing {
                break;
            }
        }

        assert_eq!(simulation.state, GameState::Playing);
        assert_eq!(simulation.player.position, spawn);
        assert_eq!(simulation.mine_count(), 0);
        assert_eq!(simulation.score, 0);
        for _ in 0..5 {
            simulation.tick(InputState::default());
            assert_eq!(simulation.state, GameState::Playing);
        }
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
    fn saturated_score_stops_awarding_extra_ships() {
        let mut simulation = Simulation::new(24);
        start(&mut simulation);
        simulation.score = u32::MAX - 1;
        simulation.next_extra_ship = u32::MAX;
        let ships = simulation.ships;

        simulation.award_points(10);

        assert_eq!(simulation.score, u32::MAX);
        assert_eq!(simulation.next_extra_ship, u32::MAX);
        assert_eq!(simulation.ships, ships);
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
        simulation.tick(InputState {
            pause: true,
            ..InputState::default()
        });
        let mut running_simulation = simulation.clone();
        running_simulation.paused = false;
        let running = running_simulation.display_list();
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
