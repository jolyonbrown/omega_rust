use std::{f32::consts::TAU, fs, io, path::Path};

use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};

use crate::{rng::Rng, sim::SfxEvent};

pub const SAMPLE_RATE: u32 = 44_100;
const AUDIO_SEED: u64 = 0x4159_2d4f_4d45_4741;
const SILENT_TAIL_SECONDS: f32 = 0.012;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SfxId {
    Fire,
    Bounce,
    EnemyExplode,
    PlayerExplode,
    MinePop,
    BulletFizzle,
    Promote,
    ExtraShip,
    WaveClear,
    FleetBonus,
    MineSweep,
    Thrust,
    Convoy(u8),
}

#[derive(Clone, Debug)]
struct SfxClip {
    id: SfxId,
    name: &'static str,
    samples: Vec<i16>,
}

impl SfxClip {
    fn duration_seconds(&self) -> f32 {
        self.samples.len() as f32 / SAMPLE_RATE as f32
    }

    fn peak(&self) -> f32 {
        self.samples
            .iter()
            .map(|sample| sample.unsigned_abs() as f32 / i16::MAX as f32)
            .fold(0.0, f32::max)
    }

    fn wav_bytes(&self) -> Vec<u8> {
        pcm_wav(&self.samples)
    }
}

/// Window-free, deterministic bank of fully synthesized PCM effects.
#[derive(Clone, Debug)]
pub struct SfxBank {
    clips: Vec<SfxClip>,
}

impl SfxBank {
    pub fn generate() -> Self {
        let mut clips = vec![
            clip(SfxId::Fire, "fire", fire()),
            clip(SfxId::Bounce, "bounce", bounce()),
            clip(SfxId::EnemyExplode, "enemy_explode", enemy_explode()),
            clip(SfxId::PlayerExplode, "player_explode", player_explode()),
            clip(SfxId::MinePop, "mine_pop", mine_pop()),
            clip(SfxId::BulletFizzle, "bullet_fizzle", bullet_fizzle()),
            clip(SfxId::Promote, "promote", promote()),
            clip(SfxId::ExtraShip, "extra_ship", extra_ship()),
            clip(SfxId::WaveClear, "wave_clear", wave_clear()),
            clip(SfxId::FleetBonus, "fleet_bonus", fleet_bonus()),
            clip(SfxId::MineSweep, "mine_sweep", mine_sweep()),
            clip(SfxId::Thrust, "thrust", thrust()),
        ];
        for variant in 0..4 {
            const NAMES: [&str; 4] = [
                "convoy_tick_0",
                "convoy_tick_1",
                "convoy_tick_2",
                "convoy_tick_3",
            ];
            clips.push(clip(
                SfxId::Convoy(variant),
                NAMES[variant as usize],
                convoy_tick(variant),
            ));
        }
        Self { clips }
    }

    pub fn write_wavs(&self, directory: &Path) -> io::Result<()> {
        fs::create_dir_all(directory)?;
        for clip in &self.clips {
            fs::write(
                directory.join(format!("{}.wav", clip.name)),
                clip.wav_bytes(),
            )?;
        }
        Ok(())
    }

    pub fn metrics(&self) -> impl Iterator<Item = (&'static str, f32, f32)> + '_ {
        self.clips
            .iter()
            .map(|clip| (clip.name, clip.duration_seconds(), clip.peak()))
    }

    #[cfg(test)]
    fn clip(&self, id: SfxId) -> &SfxClip {
        self.clips
            .iter()
            .find(|clip| clip.id == id)
            .expect("the generated SFX bank contains every effect")
    }
}

struct LoadedClip {
    id: SfxId,
    sound: Sound,
}

/// The only type that touches macroquad audio. The simulation only emits events.
pub struct AudioPlayer {
    sounds: Vec<LoadedClip>,
    muted: bool,
    thrust_requested: bool,
    thrust_loop_playing: bool,
    thrust_volume: f32,
    player_explosion_duck: f32,
}

impl AudioPlayer {
    pub async fn load(bank: &SfxBank) -> Result<Self, String> {
        let mut sounds = Vec::with_capacity(bank.clips.len());
        for clip in &bank.clips {
            let bytes = clip.wav_bytes();
            let sound = load_sound_from_bytes(&bytes).await.map_err(|error| {
                format!("could not load synthesized {} sound: {error}", clip.name)
            })?;
            sounds.push(LoadedClip { id: clip.id, sound });
        }
        Ok(Self {
            sounds,
            muted: false,
            thrust_requested: false,
            thrust_loop_playing: false,
            thrust_volume: 0.0,
            player_explosion_duck: 0.0,
        })
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            for clip in &self.sounds {
                stop_sound(&clip.sound);
            }
            self.thrust_loop_playing = false;
            self.thrust_volume = 0.0;
        }
    }

    pub fn handle_event(&mut self, event: SfxEvent) {
        match event {
            SfxEvent::ThrustOn => {
                self.thrust_requested = true;
                return;
            }
            SfxEvent::ThrustOff => {
                self.thrust_requested = false;
                return;
            }
            SfxEvent::PlayerExplode => self.player_explosion_duck = 0.9,
            _ => {}
        }

        if self.muted {
            return;
        }
        let (id, volume) = match event {
            SfxEvent::Fire => (SfxId::Fire, 1.0),
            SfxEvent::Bounce => (SfxId::Bounce, 1.0),
            SfxEvent::EnemyBounce => (SfxId::Bounce, 0.6),
            SfxEvent::EnemyExplode => (SfxId::EnemyExplode, 1.0),
            SfxEvent::PlayerExplode => (SfxId::PlayerExplode, 1.0),
            SfxEvent::MinePop => (SfxId::MinePop, 1.0),
            SfxEvent::BulletFizzle => (SfxId::BulletFizzle, 0.65),
            SfxEvent::ExtraShip => (SfxId::ExtraShip, 1.0),
            SfxEvent::WaveClear => (SfxId::WaveClear, 1.0),
            SfxEvent::FleetBonus => (SfxId::FleetBonus, 1.0),
            SfxEvent::MineSweep => (SfxId::MineSweep, 1.0),
            SfxEvent::Promote => (SfxId::Promote, 1.0),
            SfxEvent::ConvoyTick {
                living_ships,
                wave_ships,
            } => {
                let duck = if self.player_explosion_duck > 0.0 {
                    0.5
                } else {
                    1.0
                };
                (
                    SfxId::Convoy(convoy_variant(living_ships, wave_ships)),
                    0.72 * duck,
                )
            }
            SfxEvent::ThrustOn | SfxEvent::ThrustOff => unreachable!(),
        };
        self.play(id, volume, false);
    }

    pub fn update(&mut self, frame_seconds: f32) {
        self.player_explosion_duck = (self.player_explosion_duck - frame_seconds.max(0.0)).max(0.0);

        let wants_loop = self.thrust_requested && !self.muted;
        if wants_loop && !self.thrust_loop_playing {
            self.play(SfxId::Thrust, 0.0, true);
            self.thrust_loop_playing = true;
            self.thrust_volume = 0.0;
        }

        let target = if wants_loop { 1.0 } else { 0.0 };
        let fade_step = frame_seconds.max(0.0) / 0.055;
        self.thrust_volume = approach(self.thrust_volume, target, fade_step);
        if self.thrust_loop_playing {
            set_sound_volume(self.sound(SfxId::Thrust), self.thrust_volume);
            if self.thrust_volume == 0.0 && !wants_loop {
                stop_sound(self.sound(SfxId::Thrust));
                self.thrust_loop_playing = false;
            }
        }
    }

    fn play(&self, id: SfxId, volume: f32, looped: bool) {
        play_sound(self.sound(id), PlaySoundParams { looped, volume });
    }

    fn sound(&self, id: SfxId) -> &Sound {
        &self
            .sounds
            .iter()
            .find(|clip| clip.id == id)
            .expect("the loaded SFX bank contains every effect")
            .sound
    }
}

fn convoy_variant(living_ships: u8, wave_ships: u8) -> u8 {
    if wave_ships <= 1 {
        return 3;
    }
    let eliminated = wave_ships.saturating_sub(living_ships) as f32;
    let progress = eliminated / (wave_ships - 1) as f32;
    (progress * 3.0).round().clamp(0.0, 3.0) as u8
}

fn approach(current: f32, target: f32, maximum_change: f32) -> f32 {
    current + (target - current).clamp(-maximum_change, maximum_change)
}

#[derive(Clone, Copy)]
enum SweepCurve {
    Linear,
    Exponential,
}

#[derive(Clone, Copy)]
struct PitchSweep {
    start_hz: f32,
    end_hz: f32,
    curve: SweepCurve,
}

impl PitchSweep {
    const fn linear(start_hz: f32, end_hz: f32) -> Self {
        Self {
            start_hz,
            end_hz,
            curve: SweepCurve::Linear,
        }
    }

    const fn exponential(start_hz: f32, end_hz: f32) -> Self {
        Self {
            start_hz,
            end_hz,
            curve: SweepCurve::Exponential,
        }
    }

    const fn tone(hz: f32) -> Self {
        Self::linear(hz, hz)
    }

    fn at(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self.curve {
            SweepCurve::Linear => self.start_hz + (self.end_hz - self.start_hz) * progress,
            SweepCurve::Exponential => self.start_hz * (self.end_hz / self.start_hz).powf(progress),
        }
    }
}

#[derive(Clone, Copy)]
struct AmpEnvelope {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl AmpEnvelope {
    const fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
        }
    }

    fn at(self, time: f32, duration: f32) -> f32 {
        let attack_gain = if self.attack > 0.0 && time < self.attack {
            time / self.attack
        } else {
            1.0
        };
        let decay_gain = if self.decay > 0.0 && time < self.attack + self.decay {
            let progress = ((time - self.attack) / self.decay).clamp(0.0, 1.0);
            1.0 + (self.sustain - 1.0) * progress
        } else {
            self.sustain
        };
        let release_gain = if self.release > 0.0 && time > duration - self.release {
            ((duration - time) / self.release).clamp(0.0, 1.0)
        } else {
            1.0
        };
        attack_gain.min(decay_gain) * release_gain
    }
}

struct Synth {
    samples: Vec<f32>,
}

impl Synth {
    fn new(duration: f32) -> Self {
        Self {
            samples: vec![0.0; sample_count(duration)],
        }
    }

    fn square(
        &mut self,
        start: f32,
        duration: f32,
        pitch: PitchSweep,
        gain: f32,
        envelope: AmpEnvelope,
    ) {
        let start_sample = sample_count(start);
        let layer_samples = sample_count(duration);
        let end_sample = (start_sample + layer_samples).min(self.samples.len());
        let mut phase = 0.0_f32;
        for (local_sample, output) in self.samples[start_sample..end_sample]
            .iter_mut()
            .enumerate()
        {
            let time = local_sample as f32 / SAMPLE_RATE as f32;
            let progress = time / duration;
            let wave = if phase < 0.5 { 1.0 } else { -1.0 };
            *output += wave * gain * envelope.at(time, duration);
            phase = (phase + pitch.at(progress) / SAMPLE_RATE as f32).fract();
        }
    }

    fn noise(
        &mut self,
        start: f32,
        duration: f32,
        cutoff: PitchSweep,
        gain: f32,
        envelope: AmpEnvelope,
        seed: u64,
    ) {
        let start_sample = sample_count(start);
        let layer_samples = sample_count(duration);
        let end_sample = (start_sample + layer_samples).min(self.samples.len());
        let mut rng = Rng::new(AUDIO_SEED ^ seed);
        let mut filtered = 0.0_f32;
        for (local_sample, output) in self.samples[start_sample..end_sample]
            .iter_mut()
            .enumerate()
        {
            let time = local_sample as f32 / SAMPLE_RATE as f32;
            let progress = time / duration;
            let white = rng.next_f32() * 2.0 - 1.0;
            let alpha = 1.0 - (-TAU * cutoff.at(progress) / SAMPLE_RATE as f32).exp();
            filtered += alpha * (white - filtered);
            *output += filtered * gain * envelope.at(time, duration);
        }
    }

    fn finish(mut self, target_peak: f32) -> Vec<i16> {
        let peak = self
            .samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0, f32::max);
        let scale = if peak > 0.0 { target_peak / peak } else { 1.0 };
        self.samples
            .drain(..)
            .map(|sample| {
                (sample * scale)
                    .clamp(-1.0, 1.0)
                    .mul_add(i16::MAX as f32, 0.0) as i16
            })
            .collect()
    }
}

fn sample_count(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32).round() as usize
}

fn clip(id: SfxId, name: &'static str, samples: Vec<i16>) -> SfxClip {
    SfxClip { id, name, samples }
}

fn env(attack: f32, decay: f32, sustain: f32, release: f32) -> AmpEnvelope {
    AmpEnvelope::new(attack, decay, sustain, release)
}

fn active_duration(total: f32) -> f32 {
    total - SILENT_TAIL_SECONDS
}

fn fire() -> Vec<i16> {
    let total = 0.090;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.square(
        0.0,
        active,
        PitchSweep::exponential(1_400.0, 320.0),
        1.0,
        env(0.0005, 0.012, 0.42, 0.026),
    );
    synth.noise(
        0.0,
        0.014,
        PitchSweep::linear(7_500.0, 2_200.0),
        0.22,
        env(0.0002, 0.004, 0.25, 0.006),
        1,
    );
    synth.finish(0.28)
}

fn bounce() -> Vec<i16> {
    let total = 0.040;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.square(
        0.0,
        active,
        PitchSweep::linear(2_250.0, 1_850.0),
        1.0,
        env(0.0002, 0.006, 0.32, 0.012),
    );
    synth.noise(
        0.0,
        0.007,
        PitchSweep::tone(8_000.0),
        0.3,
        env(0.0001, 0.002, 0.2, 0.003),
        2,
    );
    synth.finish(0.22)
}

fn enemy_explode() -> Vec<i16> {
    let total = 0.250;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.noise(
        0.0,
        active,
        PitchSweep::exponential(7_000.0, 320.0),
        1.0,
        env(0.0008, 0.060, 0.42, 0.055),
        3,
    );
    synth.square(
        0.0,
        active,
        PitchSweep::exponential(600.0, 90.0),
        0.65,
        env(0.0008, 0.050, 0.5, 0.065),
    );
    synth.finish(0.35)
}

fn player_explode() -> Vec<i16> {
    let total = 0.900;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.noise(
        0.0,
        active,
        PitchSweep::exponential(4_800.0, 150.0),
        1.0,
        env(0.001, 0.160, 0.48, 0.180),
        4,
    );
    synth.square(
        0.0,
        active,
        PitchSweep::exponential(400.0, 40.0),
        0.66,
        env(0.001, 0.180, 0.58, 0.220),
    );
    synth.noise(
        0.300,
        active - 0.300,
        PitchSweep::exponential(1_900.0, 110.0),
        0.7,
        env(0.002, 0.100, 0.52, 0.220),
        5,
    );
    synth.finish(0.48)
}

fn mine_pop() -> Vec<i16> {
    let total = 0.060;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.square(
        0.0,
        active,
        PitchSweep::exponential(900.0, 390.0),
        1.0,
        env(0.0004, 0.009, 0.45, 0.018),
    );
    synth.square(
        0.0,
        active,
        PitchSweep::exponential(450.0, 195.0),
        -0.3,
        env(0.0004, 0.010, 0.4, 0.018),
    );
    synth.finish(0.24)
}

fn bullet_fizzle() -> Vec<i16> {
    let total = 0.030;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.square(
        0.0,
        active,
        PitchSweep::linear(2_700.0, 1_500.0),
        0.75,
        env(0.0002, 0.003, 0.24, 0.007),
    );
    synth.noise(
        0.0,
        0.008,
        PitchSweep::tone(6_500.0),
        0.45,
        env(0.0001, 0.002, 0.2, 0.003),
        6,
    );
    synth.finish(0.12)
}

fn promote() -> Vec<i16> {
    let total = 0.280;
    let mut synth = Synth::new(total);
    for (start, frequency) in [(0.0, 220.0), (0.125, 330.0)] {
        synth.square(
            start,
            0.130,
            PitchSweep::exponential(frequency * 0.92, frequency),
            1.0,
            env(0.001, 0.025, 0.62, 0.035),
        );
        synth.square(
            start,
            0.130,
            PitchSweep::tone(frequency * 0.5),
            0.28,
            env(0.001, 0.025, 0.55, 0.035),
        );
    }
    synth.finish(0.28)
}

fn extra_ship() -> Vec<i16> {
    let total = 0.450;
    let mut synth = Synth::new(total);
    for (index, frequency) in [440.0, 554.37, 659.25, 880.0].into_iter().enumerate() {
        synth.square(
            index as f32 * 0.105,
            0.123,
            PitchSweep::tone(frequency),
            1.0,
            env(0.001, 0.025, 0.63, 0.028),
        );
    }
    synth.finish(0.30)
}

fn wave_clear() -> Vec<i16> {
    let total = 0.500;
    let mut synth = Synth::new(total);
    for (start, frequency) in [(0.0, 392.0), (0.145, 523.25), (0.290, 783.99)] {
        synth.square(
            start,
            0.190,
            PitchSweep::tone(frequency),
            1.0,
            env(0.001, 0.035, 0.68, 0.045),
        );
    }
    synth.finish(0.33)
}

fn fleet_bonus() -> Vec<i16> {
    let total = 0.900;
    let mut synth = Synth::new(total);
    for (index, frequency) in [329.63, 440.0, 554.37, 659.25, 880.0]
        .into_iter()
        .enumerate()
    {
        synth.square(
            index as f32 * 0.140,
            0.240,
            PitchSweep::tone(frequency),
            1.0,
            env(0.001, 0.040, 0.7, 0.070),
        );
    }
    synth.noise(
        0.040,
        0.830,
        PitchSweep::linear(8_500.0, 3_500.0),
        0.22,
        env(0.002, 0.090, 0.32, 0.120),
        7,
    );
    synth.finish(0.38)
}

fn mine_sweep() -> Vec<i16> {
    let total = 0.400;
    let active = active_duration(total);
    let mut synth = Synth::new(total);
    synth.noise(
        0.0,
        active,
        PitchSweep::exponential(6_500.0, 170.0),
        1.0,
        env(0.002, 0.080, 0.46, 0.090),
        8,
    );
    synth.finish(0.22)
}

fn thrust() -> Vec<i16> {
    let total = 0.500;
    let active = total - 0.010;
    let mut synth = Synth::new(total);
    let loop_envelope = env(0.008, 0.040, 0.72, 0.018);
    synth.noise(
        0.0,
        active,
        PitchSweep::linear(260.0, 150.0),
        1.0,
        loop_envelope,
        9,
    );
    synth.square(0.0, active, PitchSweep::tone(55.0), 0.22, loop_envelope);
    synth.finish(0.24)
}

fn convoy_tick(variant: u8) -> Vec<i16> {
    let total = 0.230;
    let base = [68.0, 76.0, 85.0, 96.0][variant.min(3) as usize];
    let mut synth = Synth::new(total);
    for (index, (start, frequency)) in [(0.0, base), (0.105, base * 1.16)].into_iter().enumerate() {
        synth.square(
            start,
            0.082,
            PitchSweep::exponential(frequency * 1.18, frequency * 0.78),
            1.0,
            env(0.0008, 0.018, 0.44, 0.030),
        );
        synth.noise(
            start,
            0.030,
            PitchSweep::linear(1_200.0, 240.0),
            0.42,
            env(0.0004, 0.008, 0.28, 0.012),
            20 + index as u64 + variant as u64 * 3,
        );
    }
    synth.finish(0.22)
}

fn pcm_wav(samples: &[i16]) -> Vec<u8> {
    let data_bytes = samples.len() as u32 * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::{convoy_variant, pcm_wav, SfxBank, SfxId, SAMPLE_RATE};

    #[test]
    fn generated_effects_meet_the_signal_contract() {
        let bank = SfxBank::generate();
        let onset_limit = (SAMPLE_RATE as f32 * 0.005) as usize;
        let tail_samples = (SAMPLE_RATE as f32 * 0.010) as usize;
        let onset_threshold = (i16::MAX as f32 * 0.02) as u16;
        let tail_threshold = onset_threshold;

        for clip in &bank.clips {
            let peak = clip
                .samples
                .iter()
                .map(|sample| sample.unsigned_abs())
                .max()
                .unwrap();
            assert!(
                peak > (i16::MAX as f32 * 0.10) as u16,
                "{} peak was only {:.1}% FS",
                clip.name,
                peak as f32 / i16::MAX as f32 * 100.0
            );
            let onset = clip
                .samples
                .iter()
                .position(|sample| sample.unsigned_abs() > onset_threshold)
                .unwrap();
            assert!(
                onset < onset_limit,
                "{} onset sample was {onset}",
                clip.name
            );
            assert!(
                clip.samples[clip.samples.len() - tail_samples..]
                    .iter()
                    .all(|sample| sample.unsigned_abs() < tail_threshold),
                "{} did not end silent",
                clip.name
            );
        }
    }

    #[test]
    fn thrust_loop_boundary_has_no_step() {
        let bank = SfxBank::generate();
        let samples = &bank.clip(SfxId::Thrust).samples;
        let first = &samples[..64];
        let last = &samples[samples.len() - 64..];
        let maximum_join_delta = first
            .iter()
            .zip(last)
            .map(|(a, b)| (*a as i32 - *b as i32).unsigned_abs())
            .max()
            .unwrap();
        assert!(maximum_join_delta < (i16::MAX as f32 * 0.05) as u32);
    }

    #[test]
    fn busiest_common_overlap_stays_below_full_scale() {
        let bank = SfxBank::generate();
        let combined_peak = bank.clip(SfxId::Fire).peak()
            + bank.clip(SfxId::Bounce).peak()
            + bank.clip(SfxId::EnemyExplode).peak();
        assert!(combined_peak < 1.0, "combined peak was {combined_peak}");
    }

    #[test]
    fn wav_header_describes_mono_44k1_sixteen_bit_pcm() {
        let samples = [0_i16, 12_345, -12_345];
        let wav = pcm_wav(&samples);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes([wav[20], wav[21]]), 1);
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1);
        assert_eq!(
            u32::from_le_bytes(wav[24..28].try_into().unwrap()),
            SAMPLE_RATE
        );
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
        assert_eq!(wav.len(), 50);
    }

    #[test]
    fn convoy_pitch_variant_rises_as_the_fleet_thins() {
        assert_eq!(convoy_variant(5, 5), 0);
        assert!(convoy_variant(3, 5) > convoy_variant(5, 5));
        assert_eq!(convoy_variant(1, 5), 3);
    }
}
