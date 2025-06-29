use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Recursively scans a directory for `.wav` files and returns a HashMap where
/// the key is the relative directory (from root) and the value is a Vec of wav file names.
pub fn scan_wav_files_map<P: AsRef<Path>>(root: P) -> HashMap<PathBuf, Vec<String>> {
    fn helper(dir: &Path, base: &Path, map: &mut HashMap<PathBuf, Vec<String>>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    helper(&path, base, map);
                } else if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("wav") {
                        let rel_dir = path
                            .parent()
                            .unwrap()
                            .strip_prefix(base)
                            .unwrap_or(Path::new(""))
                            .to_path_buf();
                        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                        map.entry(rel_dir).or_default().push(file_name);
                    }
                }
            }
        }
    }

    let root = root.as_ref();
    let mut map = HashMap::new();
    helper(root, root, &mut map);
    map
}

// Example usage:
// let wav_map = scan_wav_files_map(r"C:\Users\dhorner\AppData\Local\SuperCollider\downloaded-quarks\Dirt-Samples");
// for (dir, files) in wav_map {
//     println!("{:?}: {:?}", dir, files);
// }

/// Holds sorted sample info for each Dirt-Samples bank, and provides lookup by name or index.
#[derive(Debug, Clone)]
pub struct DirtSampleMap {
    /// bank name (folder) -> sorted list of filenames
    pub bank_to_files: BTreeMap<String, Vec<String>>,
    /// (bank, filename) -> index
    pub file_index: HashMap<(String, String), usize>,
}

impl DirtSampleMap {
    /// Build a DirtSampleMap from a Dirt-Samples root directory.
    pub fn from_dir<P: AsRef<Path>>(root: P) -> Self {
        let mut bank_to_files: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let root = root.as_ref();
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let bank = path.file_name().unwrap().to_string_lossy().to_string();
                    let mut files: Vec<String> = Vec::new();
                    if let Ok(files_iter) = fs::read_dir(&path) {
                        for e in files_iter.flatten() {
                            let p = e.path();
                            if p.extension()
                                .map(|ext| ext.eq_ignore_ascii_case("wav"))
                                .unwrap_or(false)
                            {
                                files.push(p.file_name().unwrap().to_string_lossy().to_string());
                            }
                        }
                    }
                    files.sort();
                    if !files.is_empty() {
                        bank_to_files.insert(bank, files);
                    }
                }
            }
        }
        // Build reverse lookup
        let mut file_index = HashMap::new();
        for (bank, files) in &bank_to_files {
            for (idx, fname) in files.iter().enumerate() {
                file_index.insert((bank.clone(), fname.clone()), idx);
            }
        }
        DirtSampleMap {
            bank_to_files,
            file_index,
        }
    }

    /// Get index of a sample in a bank by filename, if present
    pub fn index_of(&self, bank: &str, filename: &str) -> Option<usize> {
        self.file_index
            .get(&(bank.to_string(), filename.to_string()))
            .copied()
    }

    /// Get filename by bank and index, if present
    pub fn filename_of(&self, bank: &str, index: usize) -> Option<&str> {
        self.bank_to_files
            .get(bank)
            .and_then(|v| v.get(index).map(|s| s.as_str()))
    }
}

/// Returns the default Dirt-Samples directory for the current user, if it exists.
/// On Windows, this is typically at:
///   C:\Users\<username>\AppData\Local\SuperCollider\downloaded-quarks\Dirt-Samples
/// On Unix-like systems, it might be:
///   ~/SuperCollider/downloaded-quarks/Dirt-Samples
pub fn default_dirt_samples_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(user_dir) = dirs::data_local_dir() {
            let path = user_dir
                .join("SuperCollider")
                .join("downloaded-quarks")
                .join("Dirt-Samples");
            if path.exists() {
                return Some(path);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home_dir) = dirs::home_dir() {
            let path = home_dir
                .join("SuperCollider")
                .join("downloaded-quarks")
                .join("Dirt-Samples");
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}
