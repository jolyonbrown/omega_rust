use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Omega Rust".to_owned(),
        window_width: 1024,
        window_height: 768,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    loop {
        clear_background(BLACK);
        draw_line(100.0, 100.0, 924.0, 668.0, 1.5, WHITE);
        next_frame().await
    }
}
