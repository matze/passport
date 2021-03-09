use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Storage {
    pub root: PathBuf,
    pub entries: Vec<PathBuf>,
}

pub struct Entry {
    pub password: String,
    pub metadata: String,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or(anyhow!("Cannot retrieve home dir"))?;
        let root = home.join(".password-store");

        fn is_gpg(entry: &walkdir::DirEntry) -> bool {
            entry
                .file_name()
                .to_str()
                .map(|s| s.ends_with(".gpg"))
                .unwrap_or(false)
        }

        let entries = walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| is_gpg(entry))
            .map(|entry| entry.into_path())
            .collect();

        Ok(Self { root, entries })
    }

    pub fn entry_name(&self, entry: &Path) -> Result<String> {
        Ok(entry
            .strip_prefix(&self.root)?
            .with_extension("")
            .to_string_lossy()
            .to_string())
    }

    pub fn decrypt(&self, entry: &str) -> Result<Entry> {
        let output = Command::new("pass")
            .arg(entry)
            .output()
            .with_context(|| format!("Could not call pass"))?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut lines = stdout.lines();
        let password = lines.next().unwrap().to_string(); // TODO: handle properly
        let metadata = lines.collect::<Vec<&str>>().join("");

        Ok(Entry { password, metadata })
    }
}
