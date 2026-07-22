use std::f32::consts::{PI, TAU};

use macroquad::math::{vec2, Vec2};

use crate::vector::Seg;

pub const MINE_CAP: usize = 24;

const CIRCUIT_LEFT: f32 = 132.0;
const CIRCUIT_RIGHT: f32 = 892.0;
const CIRCUIT_TOP: f32 = 150.0;
const CIRCUIT_BOTTOM: f32 = 642.0;
const CIRCUIT_RADIUS: f32 = 90.0;

const fn shape_seg(x1: f32, y1: f32, x2: f32, y2: f32) -> Seg {
    Seg::new(Vec2::new(x1, y1), Vec2::new(x2, y2), 1.0)
}

pub const PHOTON_MINE_SHAPE: &[Seg] = &[
    shape_seg(0.0, -5.0, 5.0, 0.0),
    shape_seg(5.0, 0.0, 0.0, 5.0),
    shape_seg(0.0, 5.0, -5.0, 0.0),
    shape_seg(-5.0, 0.0, 0.0, -5.0),
];

pub const VAPOR_MINE_SHAPE: &[Seg] = &[
    shape_seg(-8.0, -8.0, 8.0, 8.0),
    shape_seg(8.0, -8.0, -8.0, 8.0),
    shape_seg(0.0, -7.0, 0.0, 7.0),
    shape_seg(-7.0, 0.0, 7.0, 0.0),
];

pub const DROID_SHAPE: &[Seg] = &[
    shape_seg(10.0, 0.0, 2.0, -7.0),
    shape_seg(2.0, -7.0, -9.0, -5.0),
    shape_seg(-9.0, -5.0, -5.0, 0.0),
    shape_seg(-5.0, 0.0, -9.0, 5.0),
    shape_seg(-9.0, 5.0, 2.0, 7.0),
    shape_seg(2.0, 7.0, 10.0, 0.0),
    shape_seg(-4.0, -4.0, 3.0, 0.0),
    shape_seg(-4.0, 4.0, 3.0, 0.0),
];

pub const COMMAND_SHAPE: &[Seg] = &[
    shape_seg(12.0, 0.0, 1.0, -8.0),
    shape_seg(1.0, -8.0, -5.0, -4.0),
    shape_seg(-5.0, -4.0, -12.0, -7.0),
    shape_seg(-12.0, -7.0, -8.0, 0.0),
    shape_seg(-8.0, 0.0, -12.0, 7.0),
    shape_seg(-12.0, 7.0, -5.0, 4.0),
    shape_seg(-5.0, 4.0, 1.0, 8.0),
    shape_seg(1.0, 8.0, 12.0, 0.0),
    shape_seg(-8.0, 0.0, 7.0, 0.0),
];

pub const DEATH_SHAPE: &[Seg] = &[
    shape_seg(13.0, 0.0, -11.0, -6.0),
    shape_seg(-11.0, -6.0, -5.0, -1.8),
    shape_seg(-5.0, -1.8, 7.0, 0.0),
    shape_seg(7.0, 0.0, -5.0, 1.8),
    shape_seg(-5.0, 1.8, -11.0, 6.0),
    shape_seg(-11.0, 6.0, 13.0, 0.0),
    shape_seg(-11.0, -6.0, -8.0, -1.8),
    shape_seg(-11.0, 6.0, -8.0, 1.8),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    PhotonMine,
    VaporMine,
    Droid,
    Command,
    Death,
}

impl EnemyKind {
    pub const fn points(self) -> u32 {
        match self {
            Self::PhotonMine => 350,
            Self::VaporMine => 500,
            Self::Droid => 1_000,
            Self::Command => 1_500,
            Self::Death => 2_500,
        }
    }

    pub const fn radius(self) -> f32 {
        match self {
            Self::PhotonMine => 5.0,
            Self::VaporMine => 7.0,
            Self::Droid | Self::Command => 10.0,
            Self::Death => 11.0,
        }
    }

    pub const fn is_ship(self) -> bool {
        matches!(self, Self::Droid | Self::Command | Self::Death)
    }

    pub const fn is_mine(self) -> bool {
        matches!(self, Self::PhotonMine | Self::VaporMine)
    }

    pub const fn shape(self) -> &'static [Seg] {
        match self {
            Self::PhotonMine => PHOTON_MINE_SHAPE,
            Self::VaporMine => VAPOR_MINE_SHAPE,
            Self::Droid => DROID_SHAPE,
            Self::Command => COMMAND_SHAPE,
            Self::Death => DEATH_SHAPE,
        }
    }

    pub fn intensity(self, age: f32) -> f32 {
        if self == Self::VaporMine {
            let pulse = (age * TAU / 1.2).sin() * 0.5 + 0.5;
            0.6 + pulse * 0.4
        } else {
            0.9
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub age: f32,
    pub path_distance: f32,
    pub jitter: f32,
    pub action_timer: f32,
    pub mine_timer: f32,
    pub wander_phase: f32,
    pub armed: bool,
    pub armed_age: f32,
}

impl Enemy {
    pub fn droid(path_distance: f32, jitter: f32, direction: f32, speed: f32) -> Self {
        let (position, tangent, normal) = circuit_pose(path_distance);
        let position = position + normal * jitter;
        let velocity = tangent * direction * speed;
        Self {
            kind: EnemyKind::Droid,
            position,
            velocity,
            rotation: velocity.y.atan2(velocity.x),
            age: 0.0,
            path_distance,
            jitter,
            action_timer: 0.0,
            mine_timer: 0.0,
            wander_phase: path_distance * 0.013,
            armed: false,
            armed_age: 0.0,
        }
    }

    pub fn mine(kind: EnemyKind, position: Vec2, rotation: f32) -> Self {
        debug_assert!(kind.is_mine());
        Self {
            kind,
            position,
            velocity: Vec2::ZERO,
            rotation,
            age: 0.0,
            path_distance: 0.0,
            jitter: 0.0,
            action_timer: 0.0,
            mine_timer: 0.0,
            wander_phase: 0.0,
            armed: false,
            armed_age: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyBullet {
    pub position: Vec2,
    pub velocity: Vec2,
    pub ttl: Option<f32>,
}

pub fn circuit_length() -> f32 {
    let horizontal = CIRCUIT_RIGHT - CIRCUIT_LEFT - CIRCUIT_RADIUS * 2.0;
    let vertical = CIRCUIT_BOTTOM - CIRCUIT_TOP - CIRCUIT_RADIUS * 2.0;
    horizontal * 2.0 + vertical * 2.0 + TAU * CIRCUIT_RADIUS
}

/// Returns the clockwise point, unit tangent, and outward normal on the convoy
/// rounded-rectangle circuit.
pub fn circuit_pose(distance: f32) -> (Vec2, Vec2, Vec2) {
    let horizontal = CIRCUIT_RIGHT - CIRCUIT_LEFT - CIRCUIT_RADIUS * 2.0;
    let vertical = CIRCUIT_BOTTOM - CIRCUIT_TOP - CIRCUIT_RADIUS * 2.0;
    let arc = PI * 0.5 * CIRCUIT_RADIUS;
    let mut cursor = distance.rem_euclid(circuit_length());

    if cursor < horizontal {
        let point = vec2(CIRCUIT_LEFT + CIRCUIT_RADIUS + cursor, CIRCUIT_TOP);
        return (point, vec2(1.0, 0.0), vec2(0.0, -1.0));
    }
    cursor -= horizontal;
    if cursor < arc {
        return arc_pose(
            vec2(CIRCUIT_RIGHT - CIRCUIT_RADIUS, CIRCUIT_TOP + CIRCUIT_RADIUS),
            -PI * 0.5 + cursor / CIRCUIT_RADIUS,
        );
    }
    cursor -= arc;
    if cursor < vertical {
        let point = vec2(CIRCUIT_RIGHT, CIRCUIT_TOP + CIRCUIT_RADIUS + cursor);
        return (point, vec2(0.0, 1.0), vec2(1.0, 0.0));
    }
    cursor -= vertical;
    if cursor < arc {
        return arc_pose(
            vec2(
                CIRCUIT_RIGHT - CIRCUIT_RADIUS,
                CIRCUIT_BOTTOM - CIRCUIT_RADIUS,
            ),
            cursor / CIRCUIT_RADIUS,
        );
    }
    cursor -= arc;
    if cursor < horizontal {
        let point = vec2(CIRCUIT_RIGHT - CIRCUIT_RADIUS - cursor, CIRCUIT_BOTTOM);
        return (point, vec2(-1.0, 0.0), vec2(0.0, 1.0));
    }
    cursor -= horizontal;
    if cursor < arc {
        return arc_pose(
            vec2(
                CIRCUIT_LEFT + CIRCUIT_RADIUS,
                CIRCUIT_BOTTOM - CIRCUIT_RADIUS,
            ),
            PI * 0.5 + cursor / CIRCUIT_RADIUS,
        );
    }
    cursor -= arc;
    if cursor < vertical {
        let point = vec2(CIRCUIT_LEFT, CIRCUIT_BOTTOM - CIRCUIT_RADIUS - cursor);
        return (point, vec2(0.0, -1.0), vec2(-1.0, 0.0));
    }
    cursor -= vertical;
    arc_pose(
        vec2(CIRCUIT_LEFT + CIRCUIT_RADIUS, CIRCUIT_TOP + CIRCUIT_RADIUS),
        PI + cursor / CIRCUIT_RADIUS,
    )
}

fn arc_pose(center: Vec2, angle: f32) -> (Vec2, Vec2, Vec2) {
    let normal = vec2(angle.cos(), angle.sin());
    let tangent = vec2(-angle.sin(), angle.cos());
    (center + normal * CIRCUIT_RADIUS, tangent, normal)
}

#[cfg(test)]
mod tests {
    use super::{circuit_length, circuit_pose, EnemyKind};

    #[test]
    fn canonical_scoring_table_is_preserved() {
        assert_eq!(EnemyKind::PhotonMine.points(), 350);
        assert_eq!(EnemyKind::VaporMine.points(), 500);
        assert_eq!(EnemyKind::Droid.points(), 1_000);
        assert_eq!(EnemyKind::Command.points(), 1_500);
        assert_eq!(EnemyKind::Death.points(), 2_500);
    }

    #[test]
    fn rounded_circuit_wraps_without_a_seam() {
        let (start, _, _) = circuit_pose(0.0);
        let (end, _, _) = circuit_pose(circuit_length());
        assert!((start - end).length() < 0.001);
    }
}
