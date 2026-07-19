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
    use super::{is_fleet_bonus_wave, next_extra_ship_threshold, wave_size};

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
