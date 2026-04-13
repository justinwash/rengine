use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Json(serde_json::Error),
    NoSaveDir,
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "save I/O error: {e}"),
            SaveError::Json(e) => write!(f, "save serialization error: {e}"),
            SaveError::NoSaveDir => write!(f, "could not determine save directory"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        SaveError::Json(e)
    }
}

pub struct SaveSystem {
    save_dir: PathBuf,
}

impl SaveSystem {
    pub fn new(app_name: &str) -> Result<Self, SaveError> {
        let base = dirs::data_local_dir().ok_or(SaveError::NoSaveDir)?;
        let save_dir = base.join(app_name).join("saves");
        Ok(Self { save_dir })
    }

    pub fn with_dir(save_dir: PathBuf) -> Self {
        Self { save_dir }
    }

    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    pub fn save<T: Serialize>(&self, slot: &str, data: &T) -> Result<(), SaveError> {
        std::fs::create_dir_all(&self.save_dir)?;
        let path = self.slot_path(slot);
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load<T: DeserializeOwned>(&self, slot: &str) -> Result<T, SaveError> {
        let path = self.slot_path(slot);
        let text = std::fs::read_to_string(&path)?;
        let data = serde_json::from_str(&text)?;
        Ok(data)
    }

    pub fn delete(&self, slot: &str) -> Result<(), SaveError> {
        let path = self.slot_path(slot);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn exists(&self, slot: &str) -> bool {
        self.slot_path(slot).exists()
    }

    pub fn list_slots(&self) -> Vec<String> {
        let mut slots = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.save_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    if let Some(stem) = path.file_stem() {
                        slots.push(stem.to_string_lossy().into_owned());
                    }
                }
            }
        }
        slots.sort();
        slots
    }

    fn slot_path(&self, slot: &str) -> PathBuf {
        self.save_dir.join(format!("{slot}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        score: u32,
        name: String,
        position: (f32, f32),
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join("rengine_test_save_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let sys = SaveSystem::with_dir(dir.clone());

        let data = TestData {
            score: 42,
            name: "player".into(),
            position: (100.0, 200.0),
        };

        sys.save("slot1", &data).unwrap();
        assert!(sys.exists("slot1"));

        let loaded: TestData = sys.load("slot1").unwrap();
        assert_eq!(loaded, data);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_and_delete() {
        let dir = std::env::temp_dir().join("rengine_test_save_list");
        let _ = std::fs::remove_dir_all(&dir);
        let sys = SaveSystem::with_dir(dir.clone());

        sys.save("alpha", &1u32).unwrap();
        sys.save("beta", &2u32).unwrap();
        sys.save("gamma", &3u32).unwrap();

        let slots = sys.list_slots();
        assert_eq!(slots, vec!["alpha", "beta", "gamma"]);

        sys.delete("beta").unwrap();
        assert!(!sys.exists("beta"));

        let slots = sys.list_slots();
        assert_eq!(slots, vec!["alpha", "gamma"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_slot() {
        let dir = std::env::temp_dir().join("rengine_test_save_missing");
        let _ = std::fs::remove_dir_all(&dir);
        let sys = SaveSystem::with_dir(dir.clone());

        let result = sys.load::<u32>("nonexistent");
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_missing_slot_is_ok() {
        let dir = std::env::temp_dir().join("rengine_test_save_delete_ok");
        let _ = std::fs::remove_dir_all(&dir);
        let sys = SaveSystem::with_dir(dir.clone());

        assert!(sys.delete("nonexistent").is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrite_existing_slot() {
        let dir = std::env::temp_dir().join("rengine_test_save_overwrite");
        let _ = std::fs::remove_dir_all(&dir);
        let sys = SaveSystem::with_dir(dir.clone());

        sys.save("slot", &10u32).unwrap();
        sys.save("slot", &20u32).unwrap();

        let loaded: u32 = sys.load("slot").unwrap();
        assert_eq!(loaded, 20);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
