use std::f32::consts::PI;

use macroquad::math::vec2;

use crate::{
    enemies::{EnemyKind, PHOTON_MINE_SHAPE, VAPOR_MINE_SHAPE},
    font::{draw_text, draw_text_centered, text_width},
    game::{GameState, PlayPhase},
    particles,
    vector::DisplayList,
};

use super::{
    shape_seg, Simulation, BORDER_FLASH_SECONDS, BORDER_IDLE_INTENSITY, CONSOLE_BOTTOM,
    CONSOLE_LEFT, CONSOLE_RIGHT, CONSOLE_TOP, ENEMY_BULLET_HALF_LENGTH, FLEET_BONUS_SECONDS,
    OUTER_BOTTOM, OUTER_LEFT, OUTER_RIGHT, OUTER_TOP, SHIP_SHAPE, SHOT_HALF_LENGTH, TICK_RATE,
    TICK_SECONDS, WAVE_CLEARED_SECONDS,
};

const TITLE_SWEEP_SECONDS: f32 = 4.0;
const INTERSTITIAL_ZOOM_SECONDS: f32 = 0.2;
const SCORE_NORMAL_INTENSITY: f32 = 0.82;

impl Simulation {
    pub fn display_list(&self) -> DisplayList {
        let mut display_list = DisplayList::new();
        self.render_into(&mut display_list);
        display_list
    }

    pub fn render_into(&self, display_list: &mut DisplayList) {
        display_list.clear();
        match self.state {
            GameState::Attract => self.draw_attract(display_list),
            GameState::Playing | GameState::ShipDeath => self.draw_gameplay(display_list),
            GameState::GameOver => self.draw_game_over(display_list),
        }

        if self.paused {
            for segment in &mut display_list.segments {
                segment.intensity *= 0.5;
            }
            draw_text_centered(display_list, "PAUSED", vec2(512.0, 96.0), 52.0, 1.0);
        }
    }

    fn draw_attract(&self, display_list: &mut DisplayList) {
        let page = (self.state_frame / (8.0 * TICK_RATE) as u64) % 2;
        if page == 0 {
            self.draw_attract_title(display_list);
            let story = [
                "THE TIME: 2081",
                "FOR A CENTURY THE PILOTS OF OMEGA",
                "HAVE TRAINED ON THIS STATION -",
                "RACING THE VOID BETWEEN THE RAILS.",
                "NOW THE DROID ARMADA CIRCLES THE TRACK",
                "SEEDING IT WITH MINES.",
                "CLEAR THE FLEET. HOLD THE LINE.",
                "YOU ARE THE LAST OMEGAN FIGHTER.",
            ];
            for (index, line) in story.into_iter().enumerate() {
                draw_text_centered(
                    display_list,
                    line,
                    vec2(512.0, 268.0 + index as f32 * 38.0),
                    22.0,
                    0.72,
                );
            }
            draw_text_centered(
                display_list,
                &format!("HIGH SCORE {}", self.high_score),
                vec2(512.0, 620.0),
                24.0,
                0.78,
            );
        } else {
            self.draw_roster(display_list);
        }

        if self.press_enter_visible() {
            draw_text_centered(display_list, "PRESS ENTER", vec2(512.0, 702.0), 32.0, 0.95);
        }
    }

    fn draw_attract_title(&self, display_list: &mut DisplayList) {
        const TITLE: &str = "OMEGA RUST";
        const HEIGHT: f32 = 92.0;
        const BASE_INTENSITY: f32 = 0.76;

        let first_segment = display_list.segments.len();
        draw_text_centered(
            display_list,
            TITLE,
            vec2(512.0, 112.0),
            HEIGHT,
            BASE_INTENSITY,
        );

        let width = text_width(TITLE, HEIGHT);
        let left = 512.0 - width * 0.5;
        let phase = (self.state_frame as f32 * TICK_SECONDS / TITLE_SWEEP_SECONDS).fract();
        let highlight_x = left - HEIGHT + phase * (width + HEIGHT * 2.0);
        for segment in &mut display_list.segments[first_segment..] {
            let midpoint_x = (segment.a.x + segment.b.x) * 0.5;
            let highlight =
                (1.0 - (midpoint_x - highlight_x).abs() / (HEIGHT * 0.9)).clamp(0.0, 1.0);
            segment.intensity += (1.0 - segment.intensity) * highlight * highlight;
        }
    }

    fn draw_roster(&self, display_list: &mut DisplayList) {
        draw_text_centered(
            display_list,
            "KNOW YOUR ENEMY",
            vec2(512.0, 72.0),
            50.0,
            1.0,
        );
        let roster = [
            (EnemyKind::PhotonMine, "PHOTON MINE"),
            (EnemyKind::VaporMine, "VAPOR MINE"),
            (EnemyKind::Droid, "DROID SHIP"),
            (EnemyKind::Command, "COMMAND SHIP"),
            (EnemyKind::Death, "DEATH SHIP"),
        ];
        for (index, (kind, name)) in roster.into_iter().enumerate() {
            let y = 168.0 + index as f32 * 100.0;
            let rotation = if kind.is_mine() { 0.0 } else { PI };
            display_list.shape_at(
                kind.shape(),
                vec2(245.0, y),
                rotation,
                1.5,
                kind.intensity(self.state_frame as f32 * TICK_SECONDS),
            );
            draw_text(display_list, name, vec2(320.0, y - 17.0), 30.0, 0.82);
            draw_text(
                display_list,
                &kind.points().to_string(),
                vec2(760.0, y - 17.0),
                30.0,
                0.95,
            );
        }
    }

    fn press_enter_visible(&self) -> bool {
        self.state_frame % 66 < 42
    }

    fn draw_gameplay(&self, display_list: &mut DisplayList) {
        self.draw_arena(display_list);
        self.draw_console_contents(display_list);
        self.draw_enemies(display_list);
        self.draw_shots(display_list);
        self.draw_enemy_bullets(display_list);
        particles::draw(&self.particles, display_list);
        if self.state == GameState::Playing {
            self.draw_player(display_list);
        }
        self.draw_play_messages(display_list);
    }

    fn draw_game_over(&self, display_list: &mut DisplayList) {
        self.draw_arena(display_list);
        self.draw_console_contents(display_list);
        particles::draw(&self.particles, display_list);
        draw_text_centered(display_list, "GAME OVER", vec2(512.0, 112.0), 66.0, 1.0);
        draw_text_centered(
            display_list,
            &format!("FINAL SCORE {}", self.score),
            vec2(512.0, 650.0),
            30.0,
            0.86,
        );
        if self.new_high_score && self.state_frame % 36 < 24 {
            draw_text_centered(
                display_list,
                &format!("NEW HIGH SCORE {}", self.high_score),
                vec2(512.0, 704.0),
                27.0,
                1.0,
            );
        }
    }

    fn draw_play_messages(&self, display_list: &mut DisplayList) {
        match self.play_phase {
            PlayPhase::Warning => {
                draw_text_centered(
                    display_list,
                    &format!("WAVE {}", self.wave),
                    vec2(512.0, 82.0),
                    36.0,
                    0.9,
                );
                draw_text_centered(display_list, "GET READY", vec2(512.0, 126.0), 24.0, 0.65);
            }
            PlayPhase::WaveCleared => {
                let scale = interstitial_scale(self.phase_timer, WAVE_CLEARED_SECONDS);
                draw_text_centered(
                    display_list,
                    &format!("WAVE {} CLEARED", self.wave),
                    vec2(512.0, 96.0),
                    36.0 * scale,
                    1.0,
                );
            }
            PlayPhase::FleetBonus => {
                let scale = interstitial_scale(self.phase_timer, FLEET_BONUS_SECONDS);
                draw_text_centered(
                    display_list,
                    "FLEET BONUS 5000",
                    vec2(512.0, 96.0),
                    38.0 * scale,
                    1.0,
                );
            }
            PlayPhase::Active => {}
        }
        if self.extra_ship_flash > 0.0 && ((self.extra_ship_flash * 8.0) as u32).is_multiple_of(2) {
            draw_text_centered(display_list, "EXTRA SHIP", vec2(512.0, 724.0), 28.0, 1.0);
        }
    }

    fn draw_arena(&self, display_list: &mut DisplayList) {
        if self.border_flash.iter().all(|timer| *timer == 0.0) {
            display_list.closed_polygon(
                &[
                    vec2(OUTER_LEFT, OUTER_TOP),
                    vec2(OUTER_RIGHT, OUTER_TOP),
                    vec2(OUTER_RIGHT, OUTER_BOTTOM),
                    vec2(OUTER_LEFT, OUTER_BOTTOM),
                ],
                BORDER_IDLE_INTENSITY,
            );
            display_list.closed_polygon(
                &[
                    vec2(CONSOLE_LEFT, CONSOLE_TOP),
                    vec2(CONSOLE_RIGHT, CONSOLE_TOP),
                    vec2(CONSOLE_RIGHT, CONSOLE_BOTTOM),
                    vec2(CONSOLE_LEFT, CONSOLE_BOTTOM),
                ],
                BORDER_IDLE_INTENSITY,
            );
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
        draw_text_centered(
            display_list,
            &self.high_score.max(self.score).to_string(),
            vec2(center_x, 348.0),
            22.0,
            0.82,
        );
        draw_text_centered(
            display_list,
            &self.score.to_string(),
            vec2(center_x, 418.0),
            56.0,
            if self.score_flash > 0.0 {
                1.0
            } else {
                SCORE_NORMAL_INTENSITY
            },
        );

        let visible_ships = self.ships.min(9) as usize;
        let ship_spacing = 30.0;
        for index in 0..visible_ships {
            let offset = index as f32 - (visible_ships.saturating_sub(1)) as f32 * 0.5;
            display_list.shape_at(
                SHIP_SHAPE,
                vec2(center_x + offset * ship_spacing, 494.0),
                -PI * 0.5,
                0.52,
                0.72,
            );
        }
        debug_assert!(text_width("HIGH SCORE", 20.0) < CONSOLE_RIGHT - CONSOLE_LEFT);
    }

    fn draw_enemies(&self, display_list: &mut DisplayList) {
        let warning_bright = self.state_frame % 24 < 12;
        for enemy in &self.enemies {
            if self.play_phase == PlayPhase::Warning && enemy.kind.is_ship() {
                display_list.shape_at(
                    PHOTON_MINE_SHAPE,
                    enemy.position,
                    0.0,
                    1.35,
                    if warning_bright { 0.42 } else { 0.18 },
                );
                display_list.shape_at(
                    VAPOR_MINE_SHAPE,
                    enemy.position,
                    0.0,
                    0.55,
                    if warning_bright { 0.28 } else { 0.12 },
                );
            } else {
                let rotation = if enemy.kind == EnemyKind::VaporMine {
                    enemy.rotation + enemy.age * 0.28
                } else {
                    enemy.rotation
                };
                display_list.shape_at(
                    enemy.kind.shape(),
                    enemy.position,
                    rotation,
                    1.0,
                    enemy.kind.intensity(enemy.age),
                );
            }
        }
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
            if self.frame % 4 < 2 {
                let flicker = [shape_seg(
                    -9.0 - self.exhaust_length * 0.42,
                    -self.exhaust_spread * 0.28,
                    -9.0 - self.exhaust_length * 1.08,
                    self.exhaust_spread * 0.12,
                )];
                display_list.shape_at(
                    &flicker,
                    self.player.position,
                    self.player.rotation,
                    1.0,
                    0.54,
                );
            }
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

    fn draw_enemy_bullets(&self, display_list: &mut DisplayList) {
        for bullet in &self.enemy_bullets {
            let direction = bullet.velocity.normalize_or_zero();
            display_list.push_line(
                bullet.position - direction * ENEMY_BULLET_HALF_LENGTH,
                bullet.position + direction * ENEMY_BULLET_HALF_LENGTH,
                0.96,
            );
        }
    }
}

fn border_intensity(timer: f32) -> f32 {
    let flash = (timer / BORDER_FLASH_SECONDS).clamp(0.0, 1.0);
    BORDER_IDLE_INTENSITY + (1.0 - BORDER_IDLE_INTENSITY) * flash
}

fn interstitial_scale(timer: f32, duration: f32) -> f32 {
    let progress = ((duration - timer) / INTERSTITIAL_ZOOM_SECONDS).clamp(0.0, 1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    0.8 + eased * 0.2
}
