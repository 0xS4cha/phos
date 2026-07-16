use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::sort::SortField;
use crate::format::OutputFormat;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Preset {
    pub pattern: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub min_size: Option<String>,
    pub max_size: Option<String>,
    pub dirs_only: Option<bool>,
    pub files_only: Option<bool>,
    pub no_symlinks: Option<bool>,
    pub modified_within: Option<String>,
    pub modified_before: Option<String>,
    pub max_depth: Option<usize>,
    pub sort_field: Option<SortField>,
    pub reverse: Option<bool>,
    pub dirs_first: Option<bool>,
    pub output_format: Option<OutputFormat>,
    pub human_readable: Option<bool>,
    pub show_hidden: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    pub presets: HashMap<String, Preset>,
}

pub fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("phos");
        p.push("presets.toml");
        p
    })
}

pub fn load_config() -> Config {
    let path = match get_config_path() {
        Some(p) => p,
        None => return Config::default(),
    };

    if !path.exists() {
        let default_config = create_default_config();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(&default_config) {
            let _ = fs::write(&path, content);
        }
        return default_config;
    }

    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = toml::from_str::<Config>(&content) {
            return config;
        }
    }
    Config::default()
}

pub fn save_preset(name: &str, preset: Preset) -> std::io::Result<()> {
    let mut config = load_config();
    config.presets.insert(name.to_string(), preset);

    if let Some(path) = get_config_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(&config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)?;
    }
    Ok(())
}

fn create_default_config() -> Config {
    let mut presets = HashMap::new();

    presets.insert(
        "rust".to_string(),
        Preset {
            pattern: Some("*.rs".to_string()),
            sort_field: Some(SortField::Time),
            reverse: Some(true),
            output_format: Some(OutputFormat::Long),
            human_readable: Some(true),
            ..Default::default()
        },
    );

    presets.insert(
        "large".to_string(),
        Preset {
            min_size: Some("100MB".to_string()),
            sort_field: Some(SortField::Size),
            reverse: Some(true),
            output_format: Some(OutputFormat::Long),
            human_readable: Some(true),
            ..Default::default()
        },
    );

    presets.insert(
        "media".to_string(),
        Preset {
            extensions: Some(vec![
                "mp4".to_string(),
                "mkv".to_string(),
                "avi".to_string(),
                "mov".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
                "mp3".to_string(),
                "wav".to_string(),
            ]),
            sort_field: Some(SortField::Size),
            reverse: Some(true),
            output_format: Some(OutputFormat::Grid),
            ..Default::default()
        },
    );

    presets.insert(
        "recent".to_string(),
        Preset {
            modified_within: Some("1d".to_string()),
            sort_field: Some(SortField::Time),
            reverse: Some(true),
            output_format: Some(OutputFormat::Long),
            human_readable: Some(true),
            ..Default::default()
        },
    );

    Config { presets }
}

pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() { return None; }

    let value_chars: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let value: f64 = value_chars.parse().ok()?;

    let unit: String = s.chars().skip_while(|c| c.is_ascii_digit()).collect::<String>().trim().to_uppercase();
    let multiplier = match unit.as_str() {
        "B" => 1.0,
        "K" | "KB" | "KIB" => 1024.0,
        "M" | "MB" | "MIB" => 1024.0 * 1024.0,
        "G" | "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    Some((value * multiplier) as u64)
}
