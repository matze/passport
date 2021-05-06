use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct Storage {
    root: PathBuf,
}

pub struct Entry {
    pub password: String,
    pub metadata: String,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot retrieve home dir"))?;
        Ok(Self {
            root: home.join(".password-store"),
        })
    }

    pub fn entries(&self) -> Vec<String> {
        fn is_gpg(entry: &walkdir::DirEntry) -> bool {
            entry
                .file_name()
                .to_str()
                .map(|s| s.ends_with(".gpg"))
                .unwrap_or(false)
        }

        walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| is_gpg(entry))
            .map(|entry| {
                entry
                    .into_path()
                    .strip_prefix(&self.root)
                    .unwrap()
                    .with_extension("")
                    .to_string_lossy()
                    .to_string()
            })
            .collect()
    }

    pub fn decrypt(&self, entry: &str) -> Result<Entry> {
        let output = Command::new("pass")
            .arg(entry)
            .output()
            .with_context(|| "Could not call pass")?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut lines = stdout.lines();
        let password = lines.next().unwrap().to_string(); // TODO: handle properly
        let metadata = lines.collect::<Vec<&str>>().join("");

        Ok(Entry { password, metadata })
    }
}
