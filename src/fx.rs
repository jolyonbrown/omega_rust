use macroquad::{
    miniquad::{BlendFactor, BlendState, Equation},
    prelude::*,
};

use crate::{
    rng::Rng,
    sim::{VIRTUAL_HEIGHT, VIRTUAL_WIDTH},
    vector::DisplayList,
};

pub const PERSISTENCE_DECAY_60HZ: f32 = 0.72;
pub const PERSISTENCE_CAP: f32 = 0.92;
pub const GLOW_STRENGTH: f32 = 1.15;

const FRONTEND_SEED: u64 = 0x4245_414d_5f46_5831;
const FLICKER_MIN: f32 = 0.96;
const FLICKER_MAX: f32 = 1.04;
const DWELL_THRESHOLD: f32 = 0.95;
const DWELL_BOOST: f32 = 0.14;

pub struct PhosphorRenderer {
    targets: Targets,
    line_material: Material,
    persistence_material: Material,
    horizontal_blur_material: Material,
    vertical_blur_material: Material,
    composite_material: Material,
    persistence_source: usize,
    persistence_fresh: bool,
    rng: Rng,
}

impl PhosphorRenderer {
    pub fn new() -> Result<Self, String> {
        let (width, height) = letterboxed_size(screen_width() as u32, screen_height() as u32);
        Ok(Self {
            targets: Targets::new(width, height),
            line_material: load_line_material()?,
            persistence_material: load_persistence_material()?,
            horizontal_blur_material: load_blur_material(HORIZONTAL_BLUR_FRAGMENT)?,
            vertical_blur_material: load_blur_material(VERTICAL_BLUR_FRAGMENT)?,
            composite_material: load_composite_material()?,
            persistence_source: 0,
            persistence_fresh: true,
            rng: Rng::new(FRONTEND_SEED),
        })
    }

    pub fn draw(&mut self, display_list: &DisplayList, frame_seconds: f32) {
        self.resize_if_needed();
        self.draw_line_pass(display_list);
        self.update_persistence(frame_seconds);
        self.draw_glow_pass();
        self.composite_to_window();
    }

    fn resize_if_needed(&mut self) {
        let (width, height) = letterboxed_size(screen_width() as u32, screen_height() as u32);
        if (width, height) != (self.targets.width, self.targets.height) {
            self.targets = Targets::new(width, height);
            self.persistence_source = 0;
            self.persistence_fresh = true;
        }
    }

    fn draw_line_pass(&mut self, display_list: &DisplayList) {
        let mut camera = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            VIRTUAL_WIDTH as f32,
            VIRTUAL_HEIGHT as f32,
        ));
        camera.render_target = Some(self.targets.line.clone());
        set_camera(&camera);
        clear_background(BLACK);

        let pixel_scale = self.targets.width as f32 / VIRTUAL_WIDTH as f32;
        let core_width = 1.5 / pixel_scale;
        let dwell_radius = 1.25 / pixel_scale;
        let dwell_width = 0.8 / pixel_scale;
        let flicker = self.rng.range_f32(FLICKER_MIN, FLICKER_MAX);

        gl_use_material(&self.line_material);
        for segment in display_list.as_slice() {
            let intensity = (segment.intensity * flicker).clamp(0.0, 1.0);
            if intensity <= 0.0 {
                continue;
            }
            draw_line(
                segment.a.x,
                segment.a.y,
                segment.b.x,
                segment.b.y,
                core_width,
                phosphor(intensity),
            );
            if segment.intensity >= DWELL_THRESHOLD {
                for endpoint in [segment.a, segment.b] {
                    let dwell_colour = phosphor(intensity * DWELL_BOOST);
                    draw_line(
                        endpoint.x - dwell_radius,
                        endpoint.y,
                        endpoint.x + dwell_radius,
                        endpoint.y,
                        dwell_width,
                        dwell_colour,
                    );
                    draw_line(
                        endpoint.x,
                        endpoint.y - dwell_radius,
                        endpoint.x,
                        endpoint.y + dwell_radius,
                        dwell_width,
                        dwell_colour,
                    );
                }
            }
        }
        gl_use_default_material();
    }

    fn update_persistence(&mut self, frame_seconds: f32) {
        let destination = 1 - self.persistence_source;
        let decay = if self.persistence_fresh {
            0.0
        } else {
            persistence_decay(frame_seconds)
        };
        self.persistence_material.set_uniform("Decay", decay);
        self.persistence_material
            .set_uniform("PersistenceCap", PERSISTENCE_CAP);
        self.persistence_material.set_texture(
            "Previous",
            self.targets.persistence[self.persistence_source]
                .texture
                .clone(),
        );

        draw_texture_to_target(
            &self.targets.line.texture,
            &self.targets.persistence[destination],
            self.targets.width,
            self.targets.height,
            Some(&self.persistence_material),
        );
        self.persistence_source = destination;
        self.persistence_fresh = false;
    }

    fn draw_glow_pass(&self) {
        let glow_width = self.targets.glow_width;
        let glow_height = self.targets.glow_height;
        draw_texture_to_target(
            &self.targets.persistence[self.persistence_source].texture,
            &self.targets.glow_source,
            glow_width,
            glow_height,
            None,
        );

        let texel_size = vec2(1.0 / glow_width as f32, 1.0 / glow_height as f32);
        self.horizontal_blur_material
            .set_uniform("TexelSize", texel_size);
        draw_texture_to_target(
            &self.targets.glow_source.texture,
            &self.targets.blur_horizontal,
            glow_width,
            glow_height,
            Some(&self.horizontal_blur_material),
        );

        self.vertical_blur_material
            .set_uniform("TexelSize", texel_size);
        draw_texture_to_target(
            &self.targets.blur_horizontal.texture,
            &self.targets.blur_vertical,
            glow_width,
            glow_height,
            Some(&self.vertical_blur_material),
        );
    }

    fn composite_to_window(&self) {
        set_default_camera();
        clear_background(BLACK);
        let width = self.targets.width as f32;
        let height = self.targets.height as f32;
        let x = (screen_width() - width) * 0.5;
        let y = (screen_height() - height) * 0.5;

        self.composite_material
            .set_texture("Glow", self.targets.blur_vertical.texture.clone());
        self.composite_material
            .set_uniform("GlowStrength", GLOW_STRENGTH);
        gl_use_material(&self.composite_material);
        draw_texture_ex(
            &self.targets.line.texture,
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(width, height)),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();
    }
}

struct Targets {
    width: u32,
    height: u32,
    glow_width: u32,
    glow_height: u32,
    line: RenderTarget,
    persistence: [RenderTarget; 2],
    glow_source: RenderTarget,
    blur_horizontal: RenderTarget,
    blur_vertical: RenderTarget,
}

impl Targets {
    fn new(width: u32, height: u32) -> Self {
        let glow_width = (width / 4).max(1);
        let glow_height = (height / 4).max(1);
        let line = filtered_target(width, height);
        let persistence = [
            filtered_target(width, height),
            filtered_target(width, height),
        ];
        let glow_source = filtered_target(glow_width, glow_height);
        let blur_horizontal = filtered_target(glow_width, glow_height);
        let blur_vertical = filtered_target(glow_width, glow_height);
        Self {
            width,
            height,
            glow_width,
            glow_height,
            line,
            persistence,
            glow_source,
            blur_horizontal,
            blur_vertical,
        }
    }
}

fn filtered_target(width: u32, height: u32) -> RenderTarget {
    let target = render_target(width, height);
    target.texture.set_filter(FilterMode::Linear);
    target
}

fn draw_texture_to_target(
    source: &Texture2D,
    destination: &RenderTarget,
    width: u32,
    height: u32,
    material: Option<&Material>,
) {
    let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, width as f32, height as f32));
    camera.render_target = Some(destination.clone());
    set_camera(&camera);
    clear_background(BLACK);
    if let Some(material) = material {
        gl_use_material(material);
    } else {
        gl_use_default_material();
    }
    draw_texture_ex(
        source,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(width as f32, height as f32)),
            flip_y: true,
            ..Default::default()
        },
    );
    gl_use_default_material();
}

fn letterboxed_size(window_width: u32, window_height: u32) -> (u32, u32) {
    let quarter_width = window_width.max(4) / 4;
    let third_height = window_height.max(3) / 3;
    let unit = quarter_width.min(third_height).max(1);
    (unit * 4, unit * 3)
}

fn phosphor(intensity: f32) -> Color {
    Color::new(
        230.0 / 255.0 * intensity,
        240.0 / 255.0 * intensity,
        intensity,
        1.0,
    )
}

fn persistence_decay(frame_seconds: f32) -> f32 {
    PERSISTENCE_DECAY_60HZ.powf(frame_seconds.max(0.0) * 60.0)
}

fn load_line_material() -> Result<Material, String> {
    let additive = BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One);
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX_SHADER,
            fragment: LINE_FRAGMENT_SHADER,
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                color_blend: Some(additive),
                alpha_blend: Some(additive),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .map_err(|error| format!("could not load additive line material: {error}"))
}

fn load_persistence_material() -> Result<Material, String> {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX_SHADER,
            fragment: PERSISTENCE_FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("Decay", UniformType::Float1),
                UniformDesc::new("PersistenceCap", UniformType::Float1),
            ],
            textures: vec!["Previous".to_owned()],
            ..Default::default()
        },
    )
    .map_err(|error| format!("could not load persistence material: {error}"))
}

fn load_blur_material(fragment: &'static str) -> Result<Material, String> {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX_SHADER,
            fragment,
        },
        MaterialParams {
            uniforms: vec![UniformDesc::new("TexelSize", UniformType::Float2)],
            ..Default::default()
        },
    )
    .map_err(|error| format!("could not load Gaussian blur material: {error}"))
}

fn load_composite_material() -> Result<Material, String> {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX_SHADER,
            fragment: COMPOSITE_FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![UniformDesc::new("GlowStrength", UniformType::Float1)],
            textures: vec!["Glow".to_owned()],
            ..Default::default()
        },
    )
    .map_err(|error| format!("could not load glow composite material: {error}"))
}

const VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
}
"#;

const LINE_FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;

void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}
"#;

const PERSISTENCE_FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Previous;
uniform float Decay;
uniform float PersistenceCap;

void main() {
    vec3 persisted = texture2D(Texture, uv).rgb
        + texture2D(Previous, uv).rgb * Decay;
    float peak = max(max(persisted.r, persisted.g), persisted.b);
    if (peak > PersistenceCap) {
        persisted *= PersistenceCap / peak;
    }
    gl_FragColor = vec4(persisted, 1.0);
}
"#;

const HORIZONTAL_BLUR_FRAGMENT: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 TexelSize;

void main() {
    vec3 sum = texture2D(Texture, uv).rgb * 0.227027;
    sum += texture2D(Texture, uv + vec2(TexelSize.x, 0.0)).rgb * 0.194595;
    sum += texture2D(Texture, uv - vec2(TexelSize.x, 0.0)).rgb * 0.194595;
    sum += texture2D(Texture, uv + vec2(TexelSize.x * 2.0, 0.0)).rgb * 0.121622;
    sum += texture2D(Texture, uv - vec2(TexelSize.x * 2.0, 0.0)).rgb * 0.121622;
    sum += texture2D(Texture, uv + vec2(TexelSize.x * 3.0, 0.0)).rgb * 0.054054;
    sum += texture2D(Texture, uv - vec2(TexelSize.x * 3.0, 0.0)).rgb * 0.054054;
    sum += texture2D(Texture, uv + vec2(TexelSize.x * 4.0, 0.0)).rgb * 0.016216;
    sum += texture2D(Texture, uv - vec2(TexelSize.x * 4.0, 0.0)).rgb * 0.016216;
    gl_FragColor = vec4(sum, 1.0);
}
"#;

const VERTICAL_BLUR_FRAGMENT: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 TexelSize;

void main() {
    vec3 sum = texture2D(Texture, uv).rgb * 0.227027;
    sum += texture2D(Texture, uv + vec2(0.0, TexelSize.y)).rgb * 0.194595;
    sum += texture2D(Texture, uv - vec2(0.0, TexelSize.y)).rgb * 0.194595;
    sum += texture2D(Texture, uv + vec2(0.0, TexelSize.y * 2.0)).rgb * 0.121622;
    sum += texture2D(Texture, uv - vec2(0.0, TexelSize.y * 2.0)).rgb * 0.121622;
    sum += texture2D(Texture, uv + vec2(0.0, TexelSize.y * 3.0)).rgb * 0.054054;
    sum += texture2D(Texture, uv - vec2(0.0, TexelSize.y * 3.0)).rgb * 0.054054;
    sum += texture2D(Texture, uv + vec2(0.0, TexelSize.y * 4.0)).rgb * 0.016216;
    sum += texture2D(Texture, uv - vec2(0.0, TexelSize.y * 4.0)).rgb * 0.016216;
    gl_FragColor = vec4(sum, 1.0);
}
"#;

const COMPOSITE_FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D Glow;
uniform float GlowStrength;

void main() {
    vec3 crisp = texture2D(Texture, uv).rgb;
    vec3 halo = texture2D(Glow, uv).rgb;
    gl_FragColor = vec4(min(crisp + halo * GlowStrength, vec3(1.0)), 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::{letterboxed_size, persistence_decay, PERSISTENCE_DECAY_60HZ};

    #[test]
    fn letterbox_size_is_exactly_four_by_three_and_fits() {
        for (window_width, window_height) in [(1024, 768), (1600, 900), (900, 1200)] {
            let (width, height) = letterboxed_size(window_width, window_height);
            assert_eq!(width * 3, height * 4);
            assert!(width <= window_width);
            assert!(height <= window_height);
        }
    }

    #[test]
    fn persistence_decay_scales_against_a_sixty_hertz_reference() {
        assert!((persistence_decay(1.0 / 60.0) - PERSISTENCE_DECAY_60HZ).abs() < 0.000_001);
        assert!((persistence_decay(1.0 / 30.0) - PERSISTENCE_DECAY_60HZ.powi(2)).abs() < 0.000_001);
    }
}
