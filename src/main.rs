mod entry;
mod filter;
mod sort;
mod format;
mod preset;
mod engine;
mod user_cache;

use clap::Parser;
use std::path::Path;
use crate::preset::{load_config, save_preset, Preset};
use crate::sort::SortField;
use crate::format::OutputFormat;

#[derive(Parser, Debug)]
#[command(
    author = "0xS4cha",
    version = "0.1.0",
    about = "phos - High-performance ls clone with search presets in Rust"
)]
struct Args {
    #[arg(default_value = ".")]
    path: String,

    #[arg(short, long)]
    preset: Option<String>,

    #[arg(long)]
    save_preset: Option<String>,

    #[arg(short = 'P', long)]
    pattern: Option<String>,

    #[arg(short, long, value_delimiter = ',')]
    ext: Option<Vec<String>>,

    #[arg(long)]
    min_size: Option<String>,

    #[arg(long)]
    max_size: Option<String>,

    #[arg(long)]
    dirs_only: bool,

    #[arg(long)]
    files_only: bool,

    #[arg(short = 'a', long)]
    all: bool,

    #[arg(long)]
    modified_within: Option<String>,

    #[arg(long)]
    modified_before: Option<String>,

    #[arg(short = 'R', long)]
    recursive: bool,

    #[arg(long)]
    max_depth: Option<usize>,

    #[arg(long, value_enum)]
    sort: Option<CliSortField>,

    #[arg(short = 'r', long)]
    reverse: bool,

    #[arg(short = 'l', long)]
    long: bool,

    #[arg(short = 't', long)]
    tree: bool,

    #[arg(short = 'j', long)]
    json: bool,
}

#[derive(clap::ValueEnum, Clone, Debug, Copy, PartialEq, Eq)]
enum CliSortField {
    Name,
    Size,
    Time,
    Extension,
}

impl From<CliSortField> for SortField {
    fn from(f: CliSortField) -> Self {
        match f {
            CliSortField::Name => SortField::Name,
            CliSortField::Size => SortField::Size,
            CliSortField::Time => SortField::Time,
            CliSortField::Extension => SortField::Extension,
        }
    }
}

fn main() {
    let args = Args::parse();

    let mut preset = if let Some(ref preset_name) = args.preset {
        let config = load_config();
        match config.presets.get(preset_name) {
            Some(p) => p.clone(),
            None => {
                eprintln!("Error: Preset '{}' not found.", preset_name);
                eprintln!("Available presets:");
                for name in config.presets.keys() {
                    eprintln!("  - {}", name);
                }
                std::process::exit(1);
            }
        }
    } else {
        Preset::default()
    };

    if args.pattern.is_some() {
        preset.pattern = args.pattern.clone();
    }
    if args.ext.is_some() {
        preset.extensions = args.ext.clone();
    }
    if args.min_size.is_some() {
        preset.min_size = args.min_size.clone();
    }
    if args.max_size.is_some() {
        preset.max_size = args.max_size.clone();
    }
    if args.dirs_only {
        preset.dirs_only = Some(true);
    }
    if args.files_only {
        preset.files_only = Some(true);
    }
    if args.all {
        preset.show_hidden = Some(true);
    }
    if args.modified_within.is_some() {
        preset.modified_within = args.modified_within.clone();
    }
    if args.modified_before.is_some() {
        preset.modified_before = args.modified_before.clone();
    }

    if args.recursive {
        if preset.max_depth.is_none() {
            preset.max_depth = Some(usize::MAX);
        }
    }
    if let Some(depth) = args.max_depth {
        preset.max_depth = Some(depth);
    }

    if let Some(s) = args.sort {
        preset.sort_field = Some(s.into());
    }
    if args.reverse {
        preset.reverse = Some(true);
    }

    if args.long {
        preset.output_format = Some(OutputFormat::Long);
    } else if args.tree {
        preset.output_format = Some(OutputFormat::Tree);
    } else if args.json {
        preset.output_format = Some(OutputFormat::Json);
    }

    if let Some(ref save_name) = args.save_preset {
        match save_preset(save_name, preset) {
            Ok(_) => {
                println!("Successfully saved preset '{}' to config file.", save_name);
                if let Some(path) = crate::preset::get_config_path() {
                    println!("Config path: {}", path.display());
                }
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error saving preset: {}", e);
                std::process::exit(1);
            }
        }
    }

    let engine = crate::engine::Engine::new(preset.clone());
    let path = Path::new(&args.path);

    if !path.exists() {
        eprintln!("Error: Path '{}' does not exist.", args.path);
        std::process::exit(1);
    }

    let format = preset.output_format.unwrap_or(OutputFormat::Grid);

    match format {
        OutputFormat::Tree => {
            match engine.run_tree(path) {
                Ok(Some(root)) => {
                    let tree_str = crate::format::format_tree_node(&root, "", true, true);
                    print!("{}", tree_str);
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Error traversing tree: {}", e);
                    std::process::exit(1);
                }
            }
        }
        OutputFormat::Json => {
            match engine.run_flat_json(path) {
                Ok(entries) => {
                    let output = crate::format::format_json(&entries);
                    print!("{}", output);
                }
                Err(e) => {
                    eprintln!("Error reading directory: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            if let Err(e) = engine.run_flat_stream(path) {
                eprintln!("Error reading directory: {}", e);
                std::process::exit(1);
            }
        }
    }
}
