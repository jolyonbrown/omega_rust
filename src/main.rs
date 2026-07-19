mod audio;
mod enemies;
mod font;
mod fx;
mod game;
#[cfg(not(target_arch = "wasm32"))]
mod headless;
mod hiscore;
mod particles;
mod rng;
mod sim;
mod vector;

#[cfg(not(target_arch = "wasm32"))]
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use sim::{InputState, Simulation, TICK_SECONDS, VIRTUAL_HEIGHT, VIRTUAL_WIDTH};

const DEFAULT_SEED: u64 = 0x4f4d_4547_4152_5553;

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
fn main() {
    macroquad::Window::from_config(window_conf(), windowed_main());
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct HeadlessOptions {
    frames: usize,
    shot_every: Option<usize>,
    output_directory: PathBuf,
    dump_sfx_directory: Option<PathBuf>,
    seed: u64,
    script: Option<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug)]
struct ScriptRange {
    start: usize,
    end: usize,
    input: InputState,
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_headless_options(arguments: &[String]) -> Result<HeadlessOptions, String> {
    let mut frames = None;
    let mut shot_every = None;
    let mut output_directory = PathBuf::from("verify/out");
    let mut dump_sfx_directory = None;
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
            "--dump-sfx" => dump_sfx_directory = Some(PathBuf::from(value)),
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
        dump_sfx_directory,
        seed,
        script,
    })
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn run_headless(options: HeadlessOptions) -> Result<(), String> {
    if let Some(directory) = &options.dump_sfx_directory {
        let bank = audio::SfxBank::generate();
        bank.write_wavs(directory)
            .map_err(|error| format!("could not dump SFX to {}: {error}", directory.display()))?;
        println!("generated SFX:");
        for (name, duration, peak) in bank.metrics() {
            println!("  {name:<18} {duration:>5.3} s  {:>5.1}% FS", peak * 100.0);
        }
    }

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
    let mut muted = false;
    let mut previous_mute = false;
    for frame in 0..options.frames {
        let input = script_input(&script, frame);
        if input.mute && !previous_mute {
            muted = !muted;
        }
        previous_mute = input.mute;
        simulation.tick(input);
        let capture = match options.shot_every {
            Some(interval) => frame % interval == 0 || frame + 1 == options.frames,
            None => frame + 1 == options.frames,
        };
        if capture {
            let path = options
                .output_directory
                .join(format!("frame_{frame:05}.png"));
            let mut display_list = simulation.display_list();
            if muted {
                append_muted_indicator(&mut display_list);
            }
            headless::write_png(&display_list, &path)
                .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
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
                "mute" => input.mute = true,
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

#[cfg(not(target_arch = "wasm32"))]
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
    use macroquad::prelude::{get_frame_time, is_key_down, is_key_pressed, next_frame, KeyCode};

    let mut simulation = Simulation::persistent(DEFAULT_SEED);
    let generated_sfx = audio::SfxBank::generate();
    let mut audio_player = audio::AudioPlayer::load(&generated_sfx)
        .await
        .unwrap_or_else(|error| panic!("could not initialise procedural audio: {error}"));
    let mut renderer = fx::PhosphorRenderer::new()
        .unwrap_or_else(|error| panic!("could not initialise phosphor renderer: {error}"));
    let mut accumulator = 0.0_f32;
    let mut previous_mute = false;
    let mut fullscreen = false;
    loop {
        if is_key_pressed(KeyCode::F) {
            fullscreen = !fullscreen;
            macroquad::window::set_fullscreen(fullscreen);
            if !fullscreen {
                macroquad::window::request_new_screen_size(
                    VIRTUAL_WIDTH as f32,
                    VIRTUAL_HEIGHT as f32,
                );
            }
        }
        let input = InputState {
            left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
            right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            fire: is_key_down(KeyCode::Space),
            start: is_key_down(KeyCode::Enter),
            pause: is_key_down(KeyCode::P),
            mute: is_key_down(KeyCode::M),
            escape: is_key_down(KeyCode::Escape),
        };
        if input.mute && !previous_mute {
            audio_player.toggle_mute();
        }
        previous_mute = input.mute;
        let frame_seconds = get_frame_time().min(0.25);
        accumulator += frame_seconds;
        while accumulator >= TICK_SECONDS {
            simulation.tick(input);
            for event in simulation.drain_sfx_events() {
                audio_player.handle_event(event);
            }
            accumulator -= TICK_SECONDS;
        }
        audio_player.update(frame_seconds);
        if simulation.quit_requested() {
            break;
        }

        let mut display_list = simulation.display_list();
        if audio_player.is_muted() {
            append_muted_indicator(&mut display_list);
        }
        renderer.draw(&display_list, frame_seconds);
        next_frame().await;
    }
}

fn append_muted_indicator(display_list: &mut vector::DisplayList) {
    const HEIGHT: f32 = 17.0;
    const MARGIN: f32 = 14.0;
    let width = font::text_width("MUTED", HEIGHT);
    font::draw_text(
        display_list,
        "MUTED",
        macroquad::math::vec2(VIRTUAL_WIDTH as f32 - MARGIN - width, 738.0),
        HEIGHT,
        0.72,
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn print_help() {
    println!(
        "Omega Rust — M4 arcade game\n\
         \n\
         USAGE:\n\
           omega_rust                         Start the windowed game\n\
           omega_rust --headless --frames N [OPTIONS]\n\
         \n\
         HEADLESS OPTIONS:\n\
           --frames N       Number of fixed 60 Hz simulation frames (required)\n\
           --shot-every M   Save every Mth frame; default saves only the last\n\
           --out DIR        PNG directory (default: verify/out)\n\
           --dump-sfx DIR   Write all synthesized effects as WAV files\n\
           --seed S         Deterministic decimal or 0x-prefixed seed\n\
           --script FILE    Input script to run\n\
         \n\
         SCRIPT FORMAT:\n\
           START-END: key[,key...]\n\
         Ranges are 0-based and inclusive. Keys are left, right, thrust, fire,\n\
         start, pause, mute, and escape. Matching ranges are combined. Blank lines and text\n\
         after # are ignored. Example:\n\
           0-45: thrust,right\n\
           20-20: fire\n\
         \n\
         WINDOW CONTROLS:\n\
           Left/Right or A/D rotate, Up/W thrust, Space fire, Enter start,\n\
           P pause, M mute, F fullscreen, Esc back/quit"
    );
}

#[cfg(all(test, not(target_arch = "wasm32")))]
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
