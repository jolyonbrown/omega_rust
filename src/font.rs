use macroquad::math::{vec2, Vec2};

use crate::vector::{DisplayList, Seg};

#[cfg(test)]
pub const SUPPORTED_CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ.:- ";

const fn stroke(x1: f32, y1: f32, x2: f32, y2: f32) -> Seg {
    Seg::new(Vec2::new(x1, y1), Vec2::new(x2, y2), 1.0)
}

const TOP: Seg = stroke(0.08, 0.0, 0.62, 0.0);
const MIDDLE: Seg = stroke(0.08, 0.5, 0.62, 0.5);
const BOTTOM: Seg = stroke(0.08, 1.0, 0.62, 1.0);
const UPPER_LEFT: Seg = stroke(0.05, 0.04, 0.05, 0.46);
const UPPER_RIGHT: Seg = stroke(0.65, 0.04, 0.65, 0.46);
const LOWER_LEFT: Seg = stroke(0.05, 0.54, 0.05, 0.96);
const LOWER_RIGHT: Seg = stroke(0.65, 0.54, 0.65, 0.96);
const CENTER: Seg = stroke(0.35, 0.04, 0.35, 0.96);

const ZERO: &[Seg] = &[
    TOP,
    UPPER_RIGHT,
    LOWER_RIGHT,
    BOTTOM,
    LOWER_LEFT,
    UPPER_LEFT,
];
const ONE: &[Seg] = &[
    stroke(0.18, 0.18, 0.35, 0.0),
    stroke(0.35, 0.0, 0.35, 1.0),
    stroke(0.15, 1.0, 0.55, 1.0),
];
const TWO: &[Seg] = &[TOP, UPPER_RIGHT, MIDDLE, LOWER_LEFT, BOTTOM];
const THREE: &[Seg] = &[TOP, UPPER_RIGHT, MIDDLE, LOWER_RIGHT, BOTTOM];
const FOUR: &[Seg] = &[UPPER_LEFT, MIDDLE, UPPER_RIGHT, LOWER_RIGHT];
const FIVE: &[Seg] = &[TOP, UPPER_LEFT, MIDDLE, LOWER_RIGHT, BOTTOM];
const SIX: &[Seg] = &[TOP, UPPER_LEFT, MIDDLE, LOWER_LEFT, LOWER_RIGHT, BOTTOM];
const SEVEN: &[Seg] = &[TOP, UPPER_RIGHT, LOWER_RIGHT];
const EIGHT: &[Seg] = &[
    TOP,
    UPPER_LEFT,
    UPPER_RIGHT,
    MIDDLE,
    LOWER_LEFT,
    LOWER_RIGHT,
    BOTTOM,
];
const NINE: &[Seg] = &[TOP, UPPER_LEFT, UPPER_RIGHT, MIDDLE, LOWER_RIGHT, BOTTOM];

const A: &[Seg] = &[
    stroke(0.03, 1.0, 0.35, 0.0),
    stroke(0.35, 0.0, 0.67, 1.0),
    stroke(0.16, 0.58, 0.54, 0.58),
];
const B: &[Seg] = &[
    UPPER_LEFT,
    LOWER_LEFT,
    TOP,
    MIDDLE,
    BOTTOM,
    UPPER_RIGHT,
    LOWER_RIGHT,
];
const C: &[Seg] = &[TOP, UPPER_LEFT, LOWER_LEFT, BOTTOM];
const D: &[Seg] = &[
    stroke(0.05, 0.0, 0.05, 1.0),
    stroke(0.05, 0.0, 0.42, 0.0),
    stroke(0.05, 1.0, 0.42, 1.0),
    stroke(0.42, 0.0, 0.65, 0.18),
    stroke(0.65, 0.18, 0.65, 0.82),
    stroke(0.65, 0.82, 0.42, 1.0),
];
const E: &[Seg] = &[TOP, UPPER_LEFT, MIDDLE, LOWER_LEFT, BOTTOM];
const F: &[Seg] = &[TOP, UPPER_LEFT, MIDDLE, LOWER_LEFT];
const G: &[Seg] = &[
    TOP,
    UPPER_LEFT,
    LOWER_LEFT,
    BOTTOM,
    LOWER_RIGHT,
    stroke(0.38, 0.52, 0.65, 0.52),
];
const H: &[Seg] = &[UPPER_LEFT, LOWER_LEFT, MIDDLE, UPPER_RIGHT, LOWER_RIGHT];
const I: &[Seg] = &[TOP, CENTER, BOTTOM];
const J: &[Seg] = &[TOP, UPPER_RIGHT, LOWER_RIGHT, BOTTOM, LOWER_LEFT];
const K: &[Seg] = &[
    UPPER_LEFT,
    LOWER_LEFT,
    stroke(0.05, 0.54, 0.64, 0.0),
    stroke(0.28, 0.34, 0.66, 1.0),
];
const L: &[Seg] = &[UPPER_LEFT, LOWER_LEFT, BOTTOM];
const M: &[Seg] = &[
    UPPER_LEFT,
    LOWER_LEFT,
    UPPER_RIGHT,
    LOWER_RIGHT,
    stroke(0.05, 0.0, 0.35, 0.48),
    stroke(0.35, 0.48, 0.65, 0.0),
];
const N: &[Seg] = &[
    UPPER_LEFT,
    LOWER_LEFT,
    UPPER_RIGHT,
    LOWER_RIGHT,
    stroke(0.05, 0.0, 0.65, 1.0),
];
const O: &[Seg] = ZERO;
const P: &[Seg] = &[TOP, UPPER_LEFT, MIDDLE, UPPER_RIGHT, LOWER_LEFT];
const Q: &[Seg] = &[
    TOP,
    UPPER_LEFT,
    LOWER_LEFT,
    UPPER_RIGHT,
    LOWER_RIGHT,
    BOTTOM,
    stroke(0.4, 0.7, 0.7, 1.05),
];
const R: &[Seg] = &[
    TOP,
    UPPER_LEFT,
    MIDDLE,
    UPPER_RIGHT,
    LOWER_LEFT,
    stroke(0.34, 0.5, 0.68, 1.0),
];
const S: &[Seg] = FIVE;
const T: &[Seg] = &[TOP, CENTER];
const U: &[Seg] = &[UPPER_LEFT, LOWER_LEFT, UPPER_RIGHT, LOWER_RIGHT, BOTTOM];
const V: &[Seg] = &[stroke(0.03, 0.0, 0.35, 1.0), stroke(0.35, 1.0, 0.67, 0.0)];
const W: &[Seg] = &[
    stroke(0.02, 0.0, 0.16, 1.0),
    stroke(0.16, 1.0, 0.35, 0.58),
    stroke(0.35, 0.58, 0.54, 1.0),
    stroke(0.54, 1.0, 0.68, 0.0),
];
const X: &[Seg] = &[stroke(0.03, 0.0, 0.67, 1.0), stroke(0.67, 0.0, 0.03, 1.0)];
const Y: &[Seg] = &[
    stroke(0.03, 0.0, 0.35, 0.5),
    stroke(0.67, 0.0, 0.35, 0.5),
    stroke(0.35, 0.5, 0.35, 1.0),
];
const Z: &[Seg] = &[TOP, stroke(0.65, 0.0, 0.05, 1.0), BOTTOM];
const DOT: &[Seg] = &[stroke(0.3, 0.96, 0.4, 0.96)];
const COLON: &[Seg] = &[stroke(0.3, 0.32, 0.4, 0.32), stroke(0.3, 0.72, 0.4, 0.72)];
const HYPHEN: &[Seg] = &[MIDDLE];
// The zero-intensity degenerate segment makes space an explicit glyph while
// preserving its blank appearance and measurable advance.
const SPACE: &[Seg] = &[Seg::new(Vec2::ZERO, Vec2::ZERO, 0.0)];

pub fn glyph(ch: char) -> Option<&'static [Seg]> {
    match ch {
        '0' => Some(ZERO),
        '1' => Some(ONE),
        '2' => Some(TWO),
        '3' => Some(THREE),
        '4' => Some(FOUR),
        '5' => Some(FIVE),
        '6' => Some(SIX),
        '7' => Some(SEVEN),
        '8' => Some(EIGHT),
        '9' => Some(NINE),
        'A' => Some(A),
        'B' => Some(B),
        'C' => Some(C),
        'D' => Some(D),
        'E' => Some(E),
        'F' => Some(F),
        'G' => Some(G),
        'H' => Some(H),
        'I' => Some(I),
        'J' => Some(J),
        'K' => Some(K),
        'L' => Some(L),
        'M' => Some(M),
        'N' => Some(N),
        'O' => Some(O),
        'P' => Some(P),
        'Q' => Some(Q),
        'R' => Some(R),
        'S' => Some(S),
        'T' => Some(T),
        'U' => Some(U),
        'V' => Some(V),
        'W' => Some(W),
        'X' => Some(X),
        'Y' => Some(Y),
        'Z' => Some(Z),
        '.' => Some(DOT),
        ':' => Some(COLON),
        '-' => Some(HYPHEN),
        ' ' => Some(SPACE),
        _ => None,
    }
}

const ADVANCE: f32 = 0.9;
const GLYPH_WIDTH: f32 = 0.7;

pub fn text_width(text: &str, char_height: f32) -> f32 {
    let glyph_count = text.chars().filter(|ch| glyph(*ch).is_some()).count();
    match glyph_count {
        0 => 0.0,
        count => ((count - 1) as f32 * ADVANCE + GLYPH_WIDTH) * char_height,
    }
}

pub fn draw_text(
    display_list: &mut DisplayList,
    text: &str,
    pos: Vec2,
    char_height: f32,
    intensity: f32,
) {
    let mut cursor = pos;
    for ch in text.chars() {
        let Some(segments) = glyph(ch) else {
            continue;
        };
        for segment in segments.iter().filter(|segment| segment.intensity > 0.0) {
            display_list.push_line(
                cursor + segment.a * char_height,
                cursor + segment.b * char_height,
                segment.intensity * intensity,
            );
        }
        cursor += vec2(ADVANCE * char_height, 0.0);
    }
}

pub fn draw_text_centered(
    display_list: &mut DisplayList,
    text: &str,
    center: Vec2,
    char_height: f32,
    intensity: f32,
) {
    let pos = vec2(
        center.x - text_width(text, char_height) * 0.5,
        center.y - char_height * 0.5,
    );
    draw_text(display_list, text, pos, char_height, intensity);
}

#[cfg(test)]
mod tests {
    use super::{glyph, SUPPORTED_CHARS};

    #[test]
    fn every_supported_character_has_a_segment_list() {
        for ch in SUPPORTED_CHARS.chars() {
            let segments = glyph(ch).unwrap_or_else(|| panic!("missing glyph {ch:?}"));
            assert!(!segments.is_empty(), "empty glyph {ch:?}");
        }
    }

    #[test]
    fn d_has_a_distinct_segment_set_from_o() {
        let d = glyph('D').expect("D glyph");
        let o = glyph('O').expect("O glyph");
        assert!(d.len() != o.len() || d.iter().any(|segment| !o.contains(segment)));
    }
}
