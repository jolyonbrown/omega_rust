mod enemies;
mod font;
mod game;
mod headless;
mod hiscore;
mod particles;
mod rng;
mod sim;
mod vector;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use sim::{InputState, Simulation, TICK_SECONDS, VIRTUAL_HEIGHT, VIRTUAL_WIDTH};

const DEFAULT_SEED: u64 = 0x4f4d_4547_4152_5553;

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();

    if arguments.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    if arguments.first().is_some_and(|arg| arg == "--headless") {
        return match parse_headless_options(&arguments[1..]).and_then(run_headless) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}\n\nRun with --help for usage.");
                ExitCode::FAILURE
            }
        };
    }

    if !arguments.is_empty() {
        eprintln!("error: unknown windowed argument: {}", arguments[0]);
        return ExitCode::FAILURE;
    }

    macroquad::Window::from_config(window_conf(), windowed_main());
    ExitCode::SUCCESS
}

#[derive(Debug)]
struct HeadlessOptions {
    frames: usize,
    shot_every: Option<usize>,
    output_directory: PathBuf,
    seed: u64,
    script: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug)]
struct ScriptRange {
    start: usize,
    end: usize,
    input: InputState,
}

fn parse_headless_options(arguments: &[String]) -> Result<HeadlessOptions, String> {
    let mut frames = None;
    let mut shot_every = None;
    let mut output_directory = PathBuf::from("verify/out");
    let mut seed = DEFAULT_SEED;
    let mut script = None;

    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        if option == "--help" || option == "-h" {
            print_help();
            return Err("help requested".to_owned());
        }
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {option}"))?;
        match option {
            "--frames" => {
                let parsed = parse_positive_usize(value, "--frames")?;
                frames = Some(parsed);
            }
            "--shot-every" => {
                shot_every = Some(parse_positive_usize(value, "--shot-every")?);
            }
            "--out" => output_directory = PathBuf::from(value),
            "--seed" => seed = parse_seed(value)?,
            "--script" => script = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown headless option: {option}")),
        }
        index += 2;
    }

    Ok(HeadlessOptions {
        frames: frames.ok_or_else(|| "--frames N is required in headless mode".to_owned())?,
        shot_every,
        output_directory,
        seed,
        script,
    })
}

fn parse_positive_usize(value: &str, option: &str) -> Result<usize, String> {
    let number = value
        .parse::<usize>()
        .map_err(|_| format!("invalid integer for {option}: {value}"))?;
    if number == 0 {
        Err(format!("{option} must be greater than zero"))
    } else {
        Ok(number)
    }
}

fn parse_seed(value: &str) -> Result<u64, String> {
    let parsed = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
    } else {
        value.parse::<u64>()
    };
    parsed.map_err(|_| format!("invalid seed: {value}"))
}

fn run_headless(options: HeadlessOptions) -> Result<(), String> {
    let script = match options.script {
        Some(path) => parse_script(&path)?,
        None => Vec::new(),
    };
    fs::create_dir_all(&options.output_directory).map_err(|error| {
        format!(
            "could not create {}: {error}",
            options.output_directory.display()
        )
    })?;

    let mut simulation = Simulation::new(options.seed);
    for frame in 0..options.frames {
        simulation.tick(script_input(&script, frame));
        let capture = match options.shot_every {
            Some(interval) => frame % interval == 0 || frame + 1 == options.frames,
            None => frame + 1 == options.frames,
        };
        if capture {
            let path = options
                .output_directory
                .join(format!("frame_{frame:05}.png"));
            headless::write_png(&simulation.display_list(), &path)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn parse_script(path: &Path) -> Result<Vec<ScriptRange>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let mut ranges = Vec::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let (range, keys) = line.split_once(':').ok_or_else(|| {
            format!(
                "{}:{line_number}: expected START-END: key[,key...]",
                path.display()
            )
        })?;
        let (start, end) = range.trim().split_once('-').ok_or_else(|| {
            format!(
                "{}:{line_number}: expected an inclusive START-END range",
                path.display()
            )
        })?;
        let start = start
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("{}:{line_number}: invalid start frame", path.display()))?;
        let end = end
            .trim()
            .parse::<usize>()
            .map_err(|_| format!("{}:{line_number}: invalid end frame", path.display()))?;
        if start > end {
            return Err(format!(
                "{}:{line_number}: range start is after its end",
                path.display()
            ));
        }

        let mut input = InputState::default();
        for key in keys.split(',').map(str::trim).filter(|key| !key.is_empty()) {
            match key.to_ascii_lowercase().as_str() {
                "left" => input.left = true,
                "right" => input.right = true,
                "thrust" => input.thrust = true,
                "fire" => input.fire = true,
                "start" => input.start = true,
                "pause" => input.pause = true,
                "escape" => input.escape = true,
                _ => {
                    return Err(format!(
                        "{}:{line_number}: unknown key {key:?}",
                        path.display()
                    ));
                }
            }
        }
        ranges.push(ScriptRange { start, end, input });
    }
    Ok(ranges)
}

fn script_input(ranges: &[ScriptRange], frame: usize) -> InputState {
    ranges
        .iter()
        .filter(|range| frame >= range.start && frame <= range.end)
        .fold(InputState::default(), |input, range| {
            input.union(range.input)
        })
}

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "Omega Rust".to_owned(),
            window_width: VIRTUAL_WIDTH as i32,
            window_height: VIRTUAL_HEIGHT as i32,
            window_resizable: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

async fn windowed_main() {
    use macroquad::prelude::{
        clear_background, draw_line, get_frame_time, is_key_down, next_frame, screen_height,
        screen_width, Color, KeyCode, BLACK,
    };

    let mut simulation = Simulation::persistent(DEFAULT_SEED);
    let mut accumulator = 0.0_f32;
    loop {
        let input = InputState {
            left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
            right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            fire: is_key_down(KeyCode::Space),
            start: is_key_down(KeyCode::Enter),
            pause: is_key_down(KeyCode::P),
            escape: is_key_down(KeyCode::Escape),
        };
        accumulator += get_frame_time().min(0.25);
        while accumulator >= TICK_SECONDS {
            simulation.tick(input);
            accumulator -= TICK_SECONDS;
        }
        if simulation.quit_requested() {
            break;
        }

        let display_list = simulation.display_list();
        clear_background(BLACK);
        let scale =
            (screen_width() / VIRTUAL_WIDTH as f32).min(screen_height() / VIRTUAL_HEIGHT as f32);
        let offset_x = (screen_width() - VIRTUAL_WIDTH as f32 * scale) * 0.5;
        let offset_y = (screen_height() - VIRTUAL_HEIGHT as f32 * scale) * 0.5;
        for segment in display_list.as_slice() {
            let intensity = segment.intensity.clamp(0.0, 1.0);
            let colour = Color::new(
                230.0 / 255.0 * intensity,
                240.0 / 255.0 * intensity,
                intensity,
                1.0,
            );
            draw_line(
                offset_x + segment.a.x * scale,
                offset_y + segment.a.y * scale,
                offset_x + segment.b.x * scale,
                offset_y + segment.b.y * scale,
                (1.5 * scale).max(1.0),
                colour,
            );
        }
        next_frame().await;
    }
}

fn print_help() {
    println!(
        "Omega Rust — M2 arcade game\n\
         \n\
         USAGE:\n\
           omega_rust                         Start the windowed game\n\
           omega_rust --headless --frames N [OPTIONS]\n\
         \n\
         HEADLESS OPTIONS:\n\
           --frames N       Number of fixed 60 Hz simulation frames (required)\n\
           --shot-every M   Save every Mth frame; default saves only the last\n\
           --out DIR        PNG directory (default: verify/out)\n\
           --seed S         Deterministic decimal or 0x-prefixed seed\n\
           --script FILE    Input script to run\n\
         \n\
         SCRIPT FORMAT:\n\
           START-END: key[,key...]\n\
         Ranges are 0-based and inclusive. Keys are left, right, thrust, fire,\n\
         start, pause, and escape. Matching ranges are combined. Blank lines and text\n\
         after # are ignored. Example:\n\
           0-45: thrust,right\n\
           20-20: fire\n\
         \n\
         WINDOW CONTROLS:\n\
           Left/Right or A/D rotate, Up/W thrust, Space fire, Enter start,\n\
           P pause, Esc back/quit"
    );
}

#[cfg(test)]
mod tests {
    use super::{script_input, InputState, ScriptRange};

    #[test]
    fn overlapping_script_ranges_union_their_keys() {
        let ranges = [
            ScriptRange {
                start: 2,
                end: 5,
                input: InputState {
                    thrust: true,
                    ..InputState::default()
                },
            },
            ScriptRange {
                start: 4,
                end: 4,
                input: InputState {
                    fire: true,
                    ..InputState::default()
                },
            },
        ];
        let input = script_input(&ranges, 4);
        assert!(input.thrust && input.fire);
    }
}
