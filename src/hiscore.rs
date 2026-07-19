pub const DEFAULT_HIGH_SCORE: u32 = 30_000;

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::{
        env, fs, io,
        path::{Path, PathBuf},
    };

    use super::DEFAULT_HIGH_SCORE;

    #[derive(Clone, Debug)]
    pub struct Storage {
        path: Option<PathBuf>,
    }

    impl Storage {
        pub fn session() -> Self {
            Self { path: None }
        }

        pub fn persistent() -> Self {
            Self {
                path: Some(path_from_environment()),
            }
        }

        pub fn load(&self) -> u32 {
            self.path
                .as_deref()
                .and_then(|path| load(path).ok())
                .unwrap_or(DEFAULT_HIGH_SCORE)
        }

        pub fn save(&self, score: u32) {
            let Some(path) = &self.path else {
                return;
            };
            if let Err(error) = save(path, score) {
                eprintln!("could not save high score to {}: {error}", path.display());
            }
        }
    }

    fn path_from_environment() -> PathBuf {
        if let Some(data_directory) = env::var_os("OMEGA_RUST_DATA_DIR") {
            return PathBuf::from(data_directory).join("hiscore");
        }
        let home = env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
        home.join(".local/share/omega_rust/hiscore")
    }

    fn load(path: &Path) -> io::Result<u32> {
        let contents = fs::read_to_string(path)?;
        contents
            .trim()
            .parse::<u32>()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    fn save(path: &Path, score: u32) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{score}\n"))
    }

    #[cfg(test)]
    mod tests {
        use std::{env, fs, sync::Mutex};

        use super::Storage;

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

            let storage = Storage::persistent();
            storage.save(123_450);
            assert_eq!(storage.load(), 123_450);
            assert_eq!(storage.path, Some(directory.join("hiscore")));

            fs::remove_dir_all(directory).expect("clean high-score test directory");
            if let Some(value) = old_value {
                env::set_var("OMEGA_RUST_DATA_DIR", value);
            } else {
                env::remove_var("OMEGA_RUST_DATA_DIR");
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::DEFAULT_HIGH_SCORE;

    #[derive(Clone, Debug)]
    pub struct Storage;

    impl Storage {
        pub fn persistent() -> Self {
            Self
        }

        pub fn load(&self) -> u32 {
            DEFAULT_HIGH_SCORE
        }

        pub fn save(&self, _score: u32) {}
    }
}

pub use platform::Storage;
