use std::path::Path;

use image::RgbImage;

use crate::{
    sim::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH},
    vector::{DisplayList, Seg},
};

const PHOSPHOR: [f32; 3] = [230.0, 240.0, 255.0];
const GLOW_RADIUS: usize = 3;
const GLOW_STRENGTH: f32 = 0.5;

pub fn rasterize(display_list: &DisplayList) -> RgbImage {
    let mut image = RgbImage::new(VIRTUAL_WIDTH, VIRTUAL_HEIGHT);
    for segment in display_list.as_slice() {
        rasterize_segment(&mut image, *segment);
    }
    add_box_glow(&mut image);
    image
}

pub fn write_png(display_list: &DisplayList, path: &Path) -> image::ImageResult<()> {
    rasterize(display_list).save(path)
}

fn rasterize_segment(image: &mut RgbImage, segment: Seg) {
    let intensity = segment.intensity.clamp(0.0, 1.0);
    if intensity <= 0.0 {
        return;
    }

    let mut x0 = segment.a.x.round() as i32;
    let mut y0 = segment.a.y.round() as i32;
    let x1 = segment.b.x.round() as i32;
    let y1 = segment.b.y.round() as i32;
    let dx = (x1 - x0).abs();
    let step_x = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let step_y = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;

    loop {
        put_phosphor_pixel(image, x0, y0, intensity);
        for (offset_x, offset_y) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            put_phosphor_pixel(image, x0 + offset_x, y0 + offset_y, intensity * 0.2);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice_error = 2 * error;
        if twice_error >= dy {
            error += dy;
            x0 += step_x;
        }
        if twice_error <= dx {
            error += dx;
            y0 += step_y;
        }
    }
}

fn put_phosphor_pixel(image: &mut RgbImage, x: i32, y: i32, intensity: f32) {
    if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 {
        return;
    }
    let colour = PHOSPHOR.map(|channel| (channel * intensity.clamp(0.0, 1.0)).round() as u8);
    let pixel = image.get_pixel_mut(x as u32, y as u32);
    for (existing, candidate) in pixel.0.iter_mut().zip(colour) {
        *existing = (*existing).max(candidate);
    }
}

fn add_box_glow(image: &mut RgbImage) {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let kernel_width = GLOW_RADIUS * 2 + 1;
    let source: Vec<f32> = image
        .as_raw()
        .chunks_exact(3)
        .map(|pixel| pixel[2] as f32 / 255.0)
        .collect();
    let mut horizontal = vec![0.0_f32; width * height];

    for y in 0..height {
        let row = y * width;
        let mut sum: f32 = source[row..row + GLOW_RADIUS.min(width - 1) + 1]
            .iter()
            .sum();
        for x in 0..width {
            horizontal[row + x] = sum / kernel_width as f32;
            if x >= GLOW_RADIUS {
                sum -= source[row + x - GLOW_RADIUS];
            }
            let entering = x + GLOW_RADIUS + 1;
            if entering < width {
                sum += source[row + entering];
            }
        }
    }

    let pixels = image.as_mut();
    for x in 0..width {
        let mut sum = 0.0;
        for y in 0..=GLOW_RADIUS.min(height - 1) {
            sum += horizontal[y * width + x];
        }
        for y in 0..height {
            let glow = sum / kernel_width as f32 * GLOW_STRENGTH;
            let pixel = &mut pixels[(y * width + x) * 3..][..3];
            for (channel, phosphor) in pixel.iter_mut().zip(PHOSPHOR) {
                *channel = (*channel as f32 + phosphor * glow)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            if y >= GLOW_RADIUS {
                sum -= horizontal[(y - GLOW_RADIUS) * width + x];
            }
            let entering = y + GLOW_RADIUS + 1;
            if entering < height {
                sum += horizontal[entering * width + x];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use macroquad::math::vec2;

    use super::rasterize;
    use crate::vector::DisplayList;

    #[test]
    fn rasterizer_lights_a_line_without_a_window() {
        let mut display_list = DisplayList::new();
        display_list.push_line(vec2(10.0, 10.0), vec2(20.0, 10.0), 1.0);
        let image = rasterize(&display_list);
        let core = image.get_pixel(15, 10).0;
        let halo = image.get_pixel(15, 12).0;
        assert!(core[2] > halo[2]);
        assert!(halo[2] > 0);
        assert_eq!(image, rasterize(&display_list));
    }
}
