use std::f32::consts::TAU;

use macroquad::math::{vec2, Vec2};

use crate::{
    rng::Rng,
    vector::{DisplayList, Seg},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Particle {
    position: Vec2,
    local_a: Vec2,
    local_b: Vec2,
    velocity: Vec2,
    rotation: f32,
    angular_velocity: f32,
    age: f32,
    lifetime: f32,
    intensity: f32,
}

pub struct ShatterSpec<'a> {
    pub shape: &'a [Seg],
    pub position: Vec2,
    pub rotation: f32,
    pub scale: f32,
    pub base_velocity: Vec2,
    pub lifetime: f32,
    pub energy: f32,
}

pub fn spawn_shatter(particles: &mut Vec<Particle>, spec: ShatterSpec<'_>, rng: &mut Rng) {
    let (sin, cos) = spec.rotation.sin_cos();
    let transform = |point: Vec2| {
        let point = point * spec.scale;
        spec.position + vec2(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
    };

    for segment in spec.shape {
        let world_a = transform(segment.a);
        let world_b = transform(segment.b);
        let midpoint = (world_a + world_b) * 0.5;
        let random_angle = rng.range_f32(0.0, TAU);
        let random_direction = vec2(random_angle.cos(), random_angle.sin());
        let outward = (midpoint - spec.position).normalize_or_zero();
        let drift = (outward * 0.7 + random_direction * 0.5).normalize_or_zero()
            * rng.range_f32(spec.energy * 0.45, spec.energy);
        particles.push(Particle {
            position: midpoint,
            local_a: world_a - midpoint,
            local_b: world_b - midpoint,
            velocity: spec.base_velocity + drift,
            rotation: 0.0,
            angular_velocity: rng.range_f32(-5.5, 5.5),
            age: 0.0,
            lifetime: spec.lifetime * rng.range_f32(0.82, 1.08),
            intensity: segment.intensity,
        });
    }
}

pub fn update(particles: &mut Vec<Particle>, seconds: f32) {
    for particle in particles.iter_mut() {
        particle.position += particle.velocity * seconds;
        particle.rotation += particle.angular_velocity * seconds;
        particle.age += seconds;
    }
    particles.retain(|particle| particle.age < particle.lifetime);
}

pub fn draw(particles: &[Particle], display_list: &mut DisplayList) {
    for particle in particles {
        let fade = (1.0 - particle.age / particle.lifetime).clamp(0.0, 1.0);
        let (sin, cos) = particle.rotation.sin_cos();
        let transform = |point: Vec2| {
            particle.position + vec2(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
        };
        display_list.push_line(
            transform(particle.local_a),
            transform(particle.local_b),
            particle.intensity * fade,
        );
    }
}
