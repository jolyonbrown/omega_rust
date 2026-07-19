use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

pub const DEFAULT_HIGH_SCORE: u32 = 30_000;

pub fn path_from_environment() -> PathBuf {
    if let Some(data_directory) = env::var_os("OMEGA_RUST_DATA_DIR") {
        return PathBuf::from(data_directory).join("hiscore");
    }
    let home = env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    home.join(".local/share/omega_rust/hiscore")
}

pub fn load(path: &Path) -> io::Result<u32> {
    let contents = fs::read_to_string(path)?;
    contents
        .trim()
        .parse::<u32>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn save(path: &Path, score: u32) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{score}\n"))
}

#[cfg(test)]
mod tests {
    use std::{env, fs, sync::Mutex};

    use super::{load, path_from_environment, save};

    static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn high_score_round_trips_through_configured_data_directory() {
        let _guard = ENVIRONMENT_LOCK.lock().expect("environment lock poisoned");
        let old_value = env::var_os("OMEGA_RUST_DATA_DIR");
        let directory = env::temp_dir().join(format!(
            "omega-rust-hiscore-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        if directory.exists() {
            fs::remove_dir_all(&directory).expect("remove stale high-score test directory");
        }
        env::set_var("OMEGA_RUST_DATA_DIR", &directory);

        let path = path_from_environment();
        save(&path, 123_450).expect("save high score");
        assert_eq!(load(&path).expect("load high score"), 123_450);
        assert_eq!(path, directory.join("hiscore"));

        fs::remove_dir_all(directory).expect("clean high-score test directory");
        if let Some(value) = old_value {
            env::set_var("OMEGA_RUST_DATA_DIR", value);
        } else {
            env::remove_var("OMEGA_RUST_DATA_DIR");
        }
    }
}
