#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Attract,
    Playing,
    ShipDeath,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayPhase {
    Warning,
    Active,
    WaveCleared,
    FleetBonus,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Difficulty {
    pub(crate) heat: f32,
    pub(crate) convoy_speed: f32,
    pub(crate) command_wander_min_speed: f32,
    pub(crate) command_wander_max_speed: f32,
    pub(crate) death_max_speed: f32,
    pub(crate) death_steering_acceleration: f32,
    pub(crate) command_fire_min_seconds: f32,
    pub(crate) command_fire_max_seconds: f32,
    pub(crate) command_aim_error_radians: f32,
    pub(crate) enemy_bullet_speed: f32,
    pub(crate) droid_mine_min_seconds: f32,
    pub(crate) droid_mine_max_seconds: f32,
    // One-off first-drop ranges preserve the legacy wave-one RNG inputs.
    pub(crate) droid_initial_mine_min_seconds: f32,
    pub(crate) droid_initial_mine_max_seconds: f32,
    pub(crate) command_mine_min_seconds: f32,
    pub(crate) command_mine_max_seconds: f32,
    // Command promotions have their own legacy first-drop range too.
    pub(crate) command_initial_mine_min_seconds: f32,
    pub(crate) command_initial_mine_max_seconds: f32,
    pub(crate) escalation_first_seconds: f32,
    pub(crate) escalation_repeat_seconds: f32,
    pub(crate) command_promotion_seconds: f32,
}

const WAVE_ONE_DIFFICULTY: Difficulty = Difficulty {
    heat: 0.0,
    convoy_speed: 140.0,
    command_wander_min_speed: 145.0,
    command_wander_max_speed: 205.0,
    death_max_speed: 300.0,
    death_steering_acceleration: 230.0,
    command_fire_min_seconds: 1.6,
    command_fire_max_seconds: 2.6,
    command_aim_error_radians: 10.0_f32.to_radians(),
    enemy_bullet_speed: 340.0,
    droid_mine_min_seconds: 3.2,
    droid_mine_max_seconds: 5.0,
    droid_initial_mine_min_seconds: 3.2,
    droid_initial_mine_max_seconds: 4.8,
    command_mine_min_seconds: 5.5,
    command_mine_max_seconds: 8.5,
    command_initial_mine_min_seconds: 5.0,
    command_initial_mine_max_seconds: 7.0,
    escalation_first_seconds: 10.0,
    escalation_repeat_seconds: 8.0,
    command_promotion_seconds: 12.0,
};

const FULL_HEAT_DIFFICULTY: Difficulty = Difficulty {
    heat: 1.0,
    convoy_speed: 205.0,
    command_wander_min_speed: 190.0,
    command_wander_max_speed: 260.0,
    death_max_speed: 360.0,
    death_steering_acceleration: 300.0,
    command_fire_min_seconds: 0.9,
    command_fire_max_seconds: 1.6,
    command_aim_error_radians: 4.0_f32.to_radians(),
    enemy_bullet_speed: 430.0,
    droid_mine_min_seconds: 2.0,
    droid_mine_max_seconds: 3.2,
    droid_initial_mine_min_seconds: 2.0,
    droid_initial_mine_max_seconds: 3.2,
    command_mine_min_seconds: 3.8,
    command_mine_max_seconds: 6.0,
    command_initial_mine_min_seconds: 3.8,
    command_initial_mine_max_seconds: 6.0,
    escalation_first_seconds: 6.0,
    escalation_repeat_seconds: 5.0,
    command_promotion_seconds: 8.0,
};

impl Difficulty {
    pub(crate) fn for_wave(wave: u32) -> Self {
        let heat = (wave.saturating_sub(1) as f32 / 12.0).min(1.0);
        Self {
            heat,
            convoy_speed: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.convoy_speed,
                FULL_HEAT_DIFFICULTY.convoy_speed,
                heat,
            ),
            command_wander_min_speed: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_wander_min_speed,
                FULL_HEAT_DIFFICULTY.command_wander_min_speed,
                heat,
            ),
            command_wander_max_speed: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_wander_max_speed,
                FULL_HEAT_DIFFICULTY.command_wander_max_speed,
                heat,
            ),
            death_max_speed: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.death_max_speed,
                FULL_HEAT_DIFFICULTY.death_max_speed,
                heat,
            ),
            death_steering_acceleration: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.death_steering_acceleration,
                FULL_HEAT_DIFFICULTY.death_steering_acceleration,
                heat,
            ),
            command_fire_min_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_fire_min_seconds,
                FULL_HEAT_DIFFICULTY.command_fire_min_seconds,
                heat,
            ),
            command_fire_max_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_fire_max_seconds,
                FULL_HEAT_DIFFICULTY.command_fire_max_seconds,
                heat,
            ),
            command_aim_error_radians: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_aim_error_radians,
                FULL_HEAT_DIFFICULTY.command_aim_error_radians,
                heat,
            ),
            enemy_bullet_speed: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.enemy_bullet_speed,
                FULL_HEAT_DIFFICULTY.enemy_bullet_speed,
                heat,
            ),
            droid_mine_min_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.droid_mine_min_seconds,
                FULL_HEAT_DIFFICULTY.droid_mine_min_seconds,
                heat,
            ),
            droid_mine_max_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.droid_mine_max_seconds,
                FULL_HEAT_DIFFICULTY.droid_mine_max_seconds,
                heat,
            ),
            droid_initial_mine_min_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.droid_initial_mine_min_seconds,
                FULL_HEAT_DIFFICULTY.droid_initial_mine_min_seconds,
                heat,
            ),
            droid_initial_mine_max_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.droid_initial_mine_max_seconds,
                FULL_HEAT_DIFFICULTY.droid_initial_mine_max_seconds,
                heat,
            ),
            command_mine_min_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_mine_min_seconds,
                FULL_HEAT_DIFFICULTY.command_mine_min_seconds,
                heat,
            ),
            command_mine_max_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_mine_max_seconds,
                FULL_HEAT_DIFFICULTY.command_mine_max_seconds,
                heat,
            ),
            command_initial_mine_min_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_initial_mine_min_seconds,
                FULL_HEAT_DIFFICULTY.command_initial_mine_min_seconds,
                heat,
            ),
            command_initial_mine_max_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_initial_mine_max_seconds,
                FULL_HEAT_DIFFICULTY.command_initial_mine_max_seconds,
                heat,
            ),
            escalation_first_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.escalation_first_seconds,
                FULL_HEAT_DIFFICULTY.escalation_first_seconds,
                heat,
            ),
            escalation_repeat_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.escalation_repeat_seconds,
                FULL_HEAT_DIFFICULTY.escalation_repeat_seconds,
                heat,
            ),
            command_promotion_seconds: lerp_difficulty(
                WAVE_ONE_DIFFICULTY.command_promotion_seconds,
                FULL_HEAT_DIFFICULTY.command_promotion_seconds,
                heat,
            ),
        }
    }
}

fn lerp_difficulty(base: f32, full_heat: f32, heat: f32) -> f32 {
    if heat <= 0.0 {
        base
    } else if heat >= 1.0 {
        full_heat
    } else {
        base + (full_heat - base) * heat
    }
}

pub const fn wave_size(wave: u32) -> usize {
    let size = 4_u32.saturating_add(wave);
    if size < 10 {
        size as usize
    } else {
        10
    }
}

pub const fn is_fleet_bonus_wave(wave: u32) -> bool {
    wave > 0 && wave.is_multiple_of(4)
}

pub const fn next_extra_ship_threshold(score: u32) -> u32 {
    if score < 40_000 {
        40_000
    } else if score < 100_000 {
        100_000
    } else {
        (score / 100_000).saturating_add(1).saturating_mul(100_000)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_fleet_bonus_wave, next_extra_ship_threshold, wave_size, Difficulty};

    #[test]
    fn difficulty_heat_ramps_from_wave_one_to_thirteen_then_holds() {
        assert_eq!(Difficulty::for_wave(1).heat, 0.0);
        assert_eq!(Difficulty::for_wave(13).heat, 1.0);
        assert_eq!(Difficulty::for_wave(99).heat, 1.0);

        let mut previous = Difficulty::for_wave(1).heat;
        for wave in 2..=13 {
            let heat = Difficulty::for_wave(wave).heat;
            assert!(heat > previous, "heat did not increase at wave {wave}");
            previous = heat;
        }
    }

    #[test]
    fn difficulty_endpoints_preserve_wave_one_and_full_heat_tuning() {
        assert_eq!(
            Difficulty::for_wave(1),
            Difficulty {
                heat: 0.0,
                convoy_speed: 140.0,
                command_wander_min_speed: 145.0,
                command_wander_max_speed: 205.0,
                death_max_speed: 300.0,
                death_steering_acceleration: 230.0,
                command_fire_min_seconds: 1.6,
                command_fire_max_seconds: 2.6,
                command_aim_error_radians: 10.0_f32.to_radians(),
                enemy_bullet_speed: 340.0,
                droid_mine_min_seconds: 3.2,
                droid_mine_max_seconds: 5.0,
                droid_initial_mine_min_seconds: 3.2,
                droid_initial_mine_max_seconds: 4.8,
                command_mine_min_seconds: 5.5,
                command_mine_max_seconds: 8.5,
                command_initial_mine_min_seconds: 5.0,
                command_initial_mine_max_seconds: 7.0,
                escalation_first_seconds: 10.0,
                escalation_repeat_seconds: 8.0,
                command_promotion_seconds: 12.0,
            }
        );
        assert_eq!(
            Difficulty::for_wave(13),
            Difficulty {
                heat: 1.0,
                convoy_speed: 205.0,
                command_wander_min_speed: 190.0,
                command_wander_max_speed: 260.0,
                death_max_speed: 360.0,
                death_steering_acceleration: 300.0,
                command_fire_min_seconds: 0.9,
                command_fire_max_seconds: 1.6,
                command_aim_error_radians: 4.0_f32.to_radians(),
                enemy_bullet_speed: 430.0,
                droid_mine_min_seconds: 2.0,
                droid_mine_max_seconds: 3.2,
                droid_initial_mine_min_seconds: 2.0,
                droid_initial_mine_max_seconds: 3.2,
                command_mine_min_seconds: 3.8,
                command_mine_max_seconds: 6.0,
                command_initial_mine_min_seconds: 3.8,
                command_initial_mine_max_seconds: 6.0,
                escalation_first_seconds: 6.0,
                escalation_repeat_seconds: 5.0,
                command_promotion_seconds: 8.0,
            }
        );
    }

    #[test]
    fn wave_sizes_start_at_five_and_cap_at_ten() {
        assert_eq!(wave_size(1), 5);
        assert_eq!(wave_size(2), 6);
        assert_eq!(wave_size(6), 10);
        assert_eq!(wave_size(99), 10);
    }

    #[test]
    fn fleet_bonus_repeats_every_four_waves() {
        for wave in 1..=12 {
            assert_eq!(is_fleet_bonus_wave(wave), matches!(wave, 4 | 8 | 12));
        }
    }

    #[test]
    fn extra_ship_thresholds_match_the_canonical_sequence() {
        assert_eq!(next_extra_ship_threshold(0), 40_000);
        assert_eq!(next_extra_ship_threshold(39_999), 40_000);
        assert_eq!(next_extra_ship_threshold(40_000), 100_000);
        assert_eq!(next_extra_ship_threshold(99_999), 100_000);
        assert_eq!(next_extra_ship_threshold(100_000), 200_000);
        assert_eq!(next_extra_ship_threshold(199_999), 200_000);
        assert_eq!(next_extra_ship_threshold(200_000), 300_000);
        assert_eq!(next_extra_ship_threshold(u32::MAX - 1), u32::MAX);
        assert_eq!(next_extra_ship_threshold(u32::MAX), u32::MAX);
    }
}
