use chrono::{DateTime, Local, Duration};
use glob::Pattern;
use crate::entry::FileEntry;

pub trait FileFilter {
    fn matches(&self, entry: &FileEntry) -> bool;
}

pub struct GlobFilter {
    pub pattern: Pattern,
}

impl GlobFilter {
    pub fn new(pattern_str: &str) -> Result<Self, glob::PatternError> {
        let pattern = Pattern::new(pattern_str)?;
        Ok(Self { pattern })
    }
}

impl FileFilter for GlobFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        self.pattern.matches(&entry.name)
    }
}

pub struct ExtensionFilter {
    extensions: Vec<String>,
}

impl ExtensionFilter {
    pub fn new(exts: Vec<String>) -> Self {
        let extensions = exts.into_iter().map(|e| e.to_lowercase()).collect();
        Self { extensions }
    }
}

impl FileFilter for ExtensionFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        if entry.is_dir {
            return true;
        }
        if let Some(ext) = entry.path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            self.extensions.iter().any(|e| e == &ext_str)
        } else {
            false
        }
    }
}

pub struct SizeFilter {
    min_size: Option<u64>,
    max_size: Option<u64>,
}

impl SizeFilter {
    pub fn new(min_size: Option<u64>, max_size: Option<u64>) -> Self {
        Self { min_size, max_size }
    }
}

impl FileFilter for SizeFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        if entry.is_dir {
            return true;
        }
        if let Some(min) = self.min_size {
            if entry.size < min {
                return false;
            }
        }
        if let Some(max) = self.max_size {
            if entry.size > max {
                return false;
            }
        }
        true
    }
}

pub struct TypeFilter {
    pub dirs_only: bool,
    pub files_only: bool,
    pub no_symlinks: bool,
}

impl FileFilter for TypeFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        if self.dirs_only && !entry.is_dir {
            return false;
        }
        if self.files_only && !entry.is_file {
            return false;
        }
        if self.no_symlinks && entry.is_symlink {
            return false;
        }
        true
    }
}

pub struct ModifiedFilter {
    cutoff: DateTime<Local>,
    newer: bool,
}

impl ModifiedFilter {
    pub fn new(cutoff: DateTime<Local>, newer: bool) -> Self {
        Self { cutoff, newer }
    }
}

impl FileFilter for ModifiedFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        match entry.modified {
            Some(mod_time) => {
                if self.newer {
                    mod_time >= self.cutoff
                } else {
                    mod_time <= self.cutoff
                }
            }
            None => false,
        }
    }
}

pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() { return None; }
    
    let value_chars: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let value: i64 = value_chars.parse().ok()?;
    
    let unit: String = s.chars().skip_while(|c| c.is_ascii_digit()).collect::<String>().trim().to_lowercase();
    match unit.as_str() {
        "s" | "sec" | "second" | "seconds" => Some(Duration::seconds(value)),
        "m" | "min" | "minute" | "minutes" => Some(Duration::minutes(value)),
        "h" | "hour" | "hours" => Some(Duration::hours(value)),
        "d" | "day" | "days" => Some(Duration::days(value)),
        "w" | "week" | "weeks" => Some(Duration::weeks(value)),
        _ => None,
    }
}

pub struct CompositeFilter {
    filters: Vec<Box<dyn FileFilter>>,
}

impl CompositeFilter {
    pub fn new() -> Self {
        Self { filters: Vec::new() }
    }

    pub fn add(&mut self, filter: Box<dyn FileFilter>) {
        self.filters.push(filter);
    }
}

impl FileFilter for CompositeFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        self.filters.iter().all(|f| f.matches(entry))
    }
}

pub struct HiddenFilter;

impl FileFilter for HiddenFilter {
    fn matches(&self, entry: &FileEntry) -> bool {
        !entry.name.starts_with('.')
    }
}
