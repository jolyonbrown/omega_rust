use std::f32::consts::{LN_2, PI, TAU};

use macroquad::math::{vec2, Vec2};

use crate::{
    font::{draw_text_centered, text_width},
    rng::Rng,
    vector::{DisplayList, Seg},
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
const MAX_SHOTS: usize = 4;
const BORDER_FLASH_SECONDS: f32 = 0.3;
const BORDER_IDLE_INTENSITY: f32 = 0.15;

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
    border_flash: [f32; 8],
    rng: Rng,
    frame: u64,
    previous_fire: bool,
    previous_pause: bool,
    paused: bool,
    thrusting: bool,
    exhaust_length: f32,
    exhaust_spread: f32,
}

impl Simulation {
    pub fn new(seed: u64) -> Self {
        Self {
            player: Player {
                position: vec2(512.0, (CONSOLE_BOTTOM + OUTER_BOTTOM) * 0.5),
                velocity: Vec2::ZERO,
                rotation: -PI * 0.5,
                angular_velocity: 0.0,
            },
            shots: Vec::with_capacity(MAX_SHOTS),
            border_flash: [0.0; 8],
            rng: Rng::new(seed),
            frame: 0,
            previous_fire: false,
            previous_pause: false,
            paused: false,
            thrusting: false,
            exhaust_length: 12.0,
            exhaust_spread: 3.0,
        }
    }

    pub fn tick(&mut self, input: InputState) {
        if input.pause && !self.previous_pause {
            self.paused = !self.paused;
        }
        self.previous_pause = input.pause;

        if self.paused {
            self.previous_fire = input.fire;
            self.thrusting = false;
            self.frame = self.frame.wrapping_add(1);
            return;
        }

        for timer in &mut self.border_flash {
            *timer = (*timer - TICK_SECONDS).max(0.0);
        }

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
                self.exhaust_length = self.rng.range_f32(9.0, 16.0);
                self.exhaust_spread = self.rng.range_f32(2.0, 5.0);
            }
        }

        let drag = (-LN_2 * TICK_SECONDS / DRAG_HALF_LIFE).exp();
        self.player.velocity *= drag;
        self.player.velocity = self.player.velocity.clamp_length_max(MAX_SPEED);
        self.player.position += self.player.velocity * TICK_SECONDS;
        self.resolve_player_collisions();

        if input.fire && !self.previous_fire && self.shots.len() < MAX_SHOTS {
            self.shots.push(Shot {
                position: self.player.position + facing * 17.0,
                velocity: facing * SHOT_SPEED,
            });
        }
        self.previous_fire = input.fire;
        self.update_shots();
        self.frame = self.frame.wrapping_add(1);
    }

    #[cfg(test)]
    pub fn shot_count(&self) -> usize {
        self.shots.len()
    }

    pub fn display_list(&self) -> DisplayList {
        let mut display_list = DisplayList::new();
        self.render_into(&mut display_list);
        display_list
    }

    pub fn render_into(&self, display_list: &mut DisplayList) {
        display_list.clear();
        self.draw_arena(display_list);
        self.draw_console_contents(display_list);
        self.draw_player(display_list);
        self.draw_shots(display_list);
    }

    fn resolve_player_collisions(&mut self) {
        let player = &mut self.player;

        resolve_axis_wall(
            &mut player.position.x,
            OUTER_LEFT + SHIP_RADIUS,
            true,
            &mut player.velocity,
            vec2(1.0, 0.0),
            &mut self.border_flash[OUTER_LEFT_EDGE],
        );
        resolve_axis_wall(
            &mut player.position.x,
            OUTER_RIGHT - SHIP_RADIUS,
            false,
            &mut player.velocity,
            vec2(-1.0, 0.0),
            &mut self.border_flash[OUTER_RIGHT_EDGE],
        );
        resolve_axis_wall(
            &mut player.position.y,
            OUTER_TOP + SHIP_RADIUS,
            true,
            &mut player.velocity,
            vec2(0.0, 1.0),
            &mut self.border_flash[OUTER_TOP_EDGE],
        );
        resolve_axis_wall(
            &mut player.position.y,
            OUTER_BOTTOM - SHIP_RADIUS,
            false,
            &mut player.velocity,
            vec2(0.0, -1.0),
            &mut self.border_flash[OUTER_BOTTOM_EDGE],
        );

        let expanded_left = CONSOLE_LEFT - SHIP_RADIUS;
        let expanded_right = CONSOLE_RIGHT + SHIP_RADIUS;
        let expanded_top = CONSOLE_TOP - SHIP_RADIUS;
        let expanded_bottom = CONSOLE_BOTTOM + SHIP_RADIUS;
        let position = player.position;
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
            player.position = corrected_position;
            if player.velocity.dot(normal) < 0.0 {
                player.velocity = reflect_velocity(player.velocity, normal, RESTITUTION);
                self.border_flash[edge] = BORDER_FLASH_SECONDS;
            }
        }
    }

    fn update_shots(&mut self) {
        let mut flashes = [false; 8];
        self.shots.retain_mut(|shot| {
            shot.position += shot.velocity * TICK_SECONDS;
            let edge = shot_collision_edge(shot.position);
            if let Some(edge) = edge {
                flashes[edge] = true;
                false
            } else {
                true
            }
        });
        for (edge, hit) in flashes.into_iter().enumerate() {
            if hit {
                self.border_flash[edge] = BORDER_FLASH_SECONDS;
            }
        }
    }

    fn draw_arena(&self, display_list: &mut DisplayList) {
        let outer_points = [
            vec2(OUTER_LEFT, OUTER_TOP),
            vec2(OUTER_RIGHT, OUTER_TOP),
            vec2(OUTER_RIGHT, OUTER_BOTTOM),
            vec2(OUTER_LEFT, OUTER_BOTTOM),
        ];
        let console_points = [
            vec2(CONSOLE_LEFT, CONSOLE_TOP),
            vec2(CONSOLE_RIGHT, CONSOLE_TOP),
            vec2(CONSOLE_RIGHT, CONSOLE_BOTTOM),
            vec2(CONSOLE_LEFT, CONSOLE_BOTTOM),
        ];
        if self.border_flash.iter().all(|timer| *timer == 0.0) {
            display_list.closed_polygon(&outer_points, BORDER_IDLE_INTENSITY);
            display_list.closed_polygon(&console_points, BORDER_IDLE_INTENSITY);
            return;
        }

        let outer = [
            (vec2(OUTER_LEFT, OUTER_TOP), vec2(OUTER_RIGHT, OUTER_TOP)),
            (
                vec2(OUTER_RIGHT, OUTER_TOP),
                vec2(OUTER_RIGHT, OUTER_BOTTOM),
            ),
            (
                vec2(OUTER_RIGHT, OUTER_BOTTOM),
                vec2(OUTER_LEFT, OUTER_BOTTOM),
            ),
            (vec2(OUTER_LEFT, OUTER_BOTTOM), vec2(OUTER_LEFT, OUTER_TOP)),
        ];
        let console = [
            (
                vec2(CONSOLE_LEFT, CONSOLE_TOP),
                vec2(CONSOLE_RIGHT, CONSOLE_TOP),
            ),
            (
                vec2(CONSOLE_RIGHT, CONSOLE_TOP),
                vec2(CONSOLE_RIGHT, CONSOLE_BOTTOM),
            ),
            (
                vec2(CONSOLE_RIGHT, CONSOLE_BOTTOM),
                vec2(CONSOLE_LEFT, CONSOLE_BOTTOM),
            ),
            (
                vec2(CONSOLE_LEFT, CONSOLE_BOTTOM),
                vec2(CONSOLE_LEFT, CONSOLE_TOP),
            ),
        ];
        for (edge, (a, b)) in outer.into_iter().chain(console).enumerate() {
            display_list.push_line(a, b, border_intensity(self.border_flash[edge]));
        }
    }

    fn draw_console_contents(&self, display_list: &mut DisplayList) {
        let center_x = (CONSOLE_LEFT + CONSOLE_RIGHT) * 0.5;
        draw_text_centered(
            display_list,
            "HIGH SCORE",
            vec2(center_x, 318.0),
            20.0,
            0.62,
        );
        draw_text_centered(display_list, "30000", vec2(center_x, 348.0), 22.0, 0.82);
        draw_text_centered(display_list, "0", vec2(center_x, 418.0), 62.0, 0.96);

        let ship_scale = 0.58;
        let ship_spacing = 34.0;
        for index in 0..3 {
            display_list.shape_at(
                SHIP_SHAPE,
                vec2(center_x + (index as f32 - 1.0) * ship_spacing, 494.0),
                -PI * 0.5,
                ship_scale,
                0.72,
            );
        }

        // Keep the width calculation exercised here so all score layout remains
        // tied to the stroke font's metrics rather than a frontend font API.
        debug_assert!(text_width("HIGH SCORE", 20.0) < CONSOLE_RIGHT - CONSOLE_LEFT);
    }

    fn draw_player(&self, display_list: &mut DisplayList) {
        display_list.shape_at(
            SHIP_SHAPE,
            self.player.position,
            self.player.rotation,
            1.0,
            0.96,
        );
        if self.thrusting {
            let flame = [
                shape_seg(-8.5, -4.0, -8.5 - self.exhaust_length, -self.exhaust_spread),
                shape_seg(-8.5, 4.0, -8.5 - self.exhaust_length, self.exhaust_spread),
                shape_seg(
                    -9.0,
                    0.0,
                    -9.0 - self.exhaust_length * 0.72,
                    self.exhaust_spread * 0.2,
                ),
            ];
            display_list.shape_at(
                &flame,
                self.player.position,
                self.player.rotation,
                1.0,
                0.68,
            );
        }
    }

    fn draw_shots(&self, display_list: &mut DisplayList) {
        for shot in &self.shots {
            let direction = shot.velocity.normalize_or_zero();
            display_list.push_line(
                shot.position - direction * SHOT_HALF_LENGTH,
                shot.position + direction * SHOT_HALF_LENGTH,
                1.0,
            );
        }
    }
}

pub fn reflect_velocity(velocity: Vec2, normal: Vec2, restitution: f32) -> Vec2 {
    let normal = normal.normalize_or_zero();
    (velocity - 2.0 * velocity.dot(normal) * normal) * restitution
}

fn direction(rotation: f32) -> Vec2 {
    vec2(rotation.cos(), rotation.sin())
}

fn approach(current: f32, target: f32, maximum_change: f32) -> f32 {
    current + (target - current).clamp(-maximum_change, maximum_change)
}

fn resolve_axis_wall(
    coordinate: &mut f32,
    limit: f32,
    is_minimum: bool,
    velocity: &mut Vec2,
    inward_normal: Vec2,
    flash_timer: &mut f32,
) {
    let crossed = if is_minimum {
        *coordinate < limit
    } else {
        *coordinate > limit
    };
    if crossed {
        *coordinate = limit;
        if velocity.dot(inward_normal) < 0.0 {
            *velocity = reflect_velocity(*velocity, inward_normal, RESTITUTION);
            *flash_timer = BORDER_FLASH_SECONDS;
        }
    }
}

fn shot_collision_edge(position: Vec2) -> Option<usize> {
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

fn border_intensity(timer: f32) -> f32 {
    let flash = (timer / BORDER_FLASH_SECONDS).clamp(0.0, 1.0);
    BORDER_IDLE_INTENSITY + (1.0 - BORDER_IDLE_INTENSITY) * flash
}

#[cfg(test)]
mod tests {
    use macroquad::math::vec2;

    use super::{reflect_velocity, InputState, Simulation};

    #[test]
    fn reflection_changes_direction_and_scales_speed_by_restitution() {
        let velocity = vec2(-30.0, 40.0);
        let reflected = reflect_velocity(velocity, vec2(1.0, 0.0), 0.9);
        assert!((reflected - vec2(27.0, 36.0)).length() < 0.0001);
        assert!((reflected.length() - velocity.length() * 0.9).abs() < 0.0001);
    }

    #[test]
    fn only_four_shots_can_be_live() {
        let mut simulation = Simulation::new(7);
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
    fn equal_seed_and_input_produce_identical_display_lists() {
        fn run(seed: u64) -> crate::vector::DisplayList {
            let mut simulation = Simulation::new(seed);
            for frame in 0..240 {
                simulation.tick(InputState {
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
}
