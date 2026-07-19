use std::path::Path;

use image::RgbImage;

use crate::{
    sim::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH},
    vector::{DisplayList, Seg},
};

const PHOSPHOR: [f32; 3] = [230.0, 240.0, 255.0];

pub fn rasterize(display_list: &DisplayList) -> RgbImage {
    let mut image = RgbImage::new(VIRTUAL_WIDTH, VIRTUAL_HEIGHT);
    for segment in display_list.as_slice() {
        rasterize_segment(&mut image, *segment);
    }
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
        assert_eq!(image.get_pixel(15, 10).0, [230, 240, 255]);
        assert_eq!(image.get_pixel(15, 12).0, [0, 0, 0]);
    }
}
