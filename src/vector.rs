use macroquad::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub a: Vec2,
    pub b: Vec2,
    pub intensity: f32,
}

impl Seg {
    pub const fn new(a: Vec2, b: Vec2, intensity: f32) -> Self {
        Self { a, b, intensity }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisplayList {
    pub segments: Vec<Seg>,
}

impl DisplayList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.segments.clear();
    }

    pub fn push_line(&mut self, a: Vec2, b: Vec2, intensity: f32) {
        self.segments.push(Seg::new(a, b, intensity));
    }

    pub fn polyline(&mut self, points: &[Vec2], intensity: f32) {
        for pair in points.windows(2) {
            self.push_line(pair[0], pair[1], intensity);
        }
    }

    pub fn closed_polygon(&mut self, points: &[Vec2], intensity: f32) {
        self.polyline(points, intensity);
        if points.len() > 2 {
            self.push_line(points[points.len() - 1], points[0], intensity);
        }
    }

    /// Appends a local-space shape after applying a uniform scale, rotation,
    /// and translation. Shape and call intensities are multiplied together.
    pub fn shape_at(
        &mut self,
        shape: &[Seg],
        position: Vec2,
        rotation: f32,
        scale: f32,
        intensity: f32,
    ) {
        let (sin, cos) = rotation.sin_cos();
        let transform = |point: Vec2| {
            let point = point * scale;
            position + Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
        };

        self.segments.extend(shape.iter().map(|seg| Seg {
            a: transform(seg.a),
            b: transform(seg.b),
            intensity: (seg.intensity * intensity).clamp(0.0, 1.0),
        }));
    }

    pub fn as_slice(&self) -> &[Seg] {
        &self.segments
    }
}
