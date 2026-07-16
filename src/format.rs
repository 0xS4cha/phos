use crate::entry::FileEntry;
use colored::Colorize;
use terminal_size::{Width, Height, terminal_size};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Grid,
    Long,
    Tree,
    Json,
}

pub fn get_terminal_width() -> usize {
    if let Some((Width(w), Height(_))) = terminal_size() {
        w as usize
    } else {
        80
    }
}

pub fn colorize_name(entry: &FileEntry) -> String {
    let name = &entry.name;
    if entry.is_dir {
        name.bold().blue().to_string()
    } else if entry.is_symlink {
        name.cyan().to_string()
    } else if entry.is_file {
        #[cfg(unix)]
        let is_executable = entry.mode.map(|m| m & 0o111 != 0).unwrap_or(false);
        #[cfg(not(unix))]
        let is_executable = false;

        if is_executable {
            name.bold().green().to_string()
        } else if let Some(ext) = entry.path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "rs" | "go" | "py" | "c" | "cpp" | "h" | "hpp" | "sh" | "js" | "ts" | "toml" | "json" | "yaml" | "yml" | "md" => {
                    name.yellow().to_string()
                }
                "zip" | "tar" | "gz" | "bz2" | "xz" | "rar" | "7z" => {
                    name.red().to_string()
                }
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "mp4" | "mkv" | "avi" | "mov" | "mp3" | "flac" | "wav" => {
                    name.magenta().to_string()
                }
                _ => name.to_string(),
            }
        } else {
            name.to_string()
        }
    } else {
        name.to_string()
    }
}

pub fn format_size(size: u64, human_readable: bool) -> String {
    if !human_readable {
        return size.to_string();
    }
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.1}T", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

pub fn format_grid(entries: &[FileEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let names_with_len: Vec<(String, usize)> = entries.iter()
        .map(|entry| {
            let colored_name = colorize_name(entry);
            (colored_name, entry.name.len())
        })
        .collect();

    let max_len = names_with_len.iter().map(|(_, len)| *len).max().unwrap_or(0);
    let col_width = max_len + 2;

    let term_width = get_terminal_width();
    let cols = std::cmp::max(1, term_width / col_width);
    let rows = (names_with_len.len() + cols - 1) / cols;

    let mut output = String::new();
    for r in 0..rows {
        for c in 0..cols {
            let idx = c * rows + r;
            if idx < names_with_len.len() {
                let (colored_name, raw_len) = &names_with_len[idx];
                output.push_str(colored_name);
                if c < cols - 1 {
                    let padding = col_width - raw_len;
                    output.push_str(&" ".repeat(padding));
                }
            }
        }
        output.push('\n');
    }
    output
}

pub fn format_long(entries: &[FileEntry], human_readable: bool) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut rows = Vec::new();
    let mut max_owner_len = 0;
    let mut max_group_len = 0;
    let mut max_size_len = 0;

    for entry in entries {
        let type_char = entry.file_type_char();
        let perm = entry.permissions_str();
        let owner = entry.owner_name();
        let group = entry.group_name();
        let size_str = format_size(entry.size, human_readable);

        max_owner_len = std::cmp::max(max_owner_len, owner.len());
        max_group_len = std::cmp::max(max_group_len, group.len());
        max_size_len = std::cmp::max(max_size_len, size_str.len());

        let date_str = match entry.modified {
            Some(dt) => dt.format("%b %e %H:%M").to_string(),
            None => "Jan  1  1970".to_string(),
        };

        rows.push((type_char, perm, owner, group, size_str, date_str, entry.clone()));
    }

    let mut output = String::new();
    for (type_char, perm, owner, group, size_str, date_str, entry) in rows {
        let name_colored = colorize_name(&entry);
        let link_target = if entry.is_symlink {
            if let Ok(target) = std::fs::read_link(&entry.path) {
                format!(" -> {}", target.to_string_lossy().cyan())
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        output.push_str(&format!(
            "{}{} {:<owner_width$} {:<group_width$} {:>size_width$} {} {}{}\n",
            type_char,
            perm,
            owner,
            group,
            size_str,
            date_str,
            name_colored,
            link_target,
            owner_width = max_owner_len,
            group_width = max_group_len,
            size_width = max_size_len
        ));
    }
    output
}

pub fn format_json(entries: &[FileEntry]) -> String {
    #[derive(Serialize)]
    struct JsonEntry {
        name: String,
        path: String,
        is_dir: bool,
        is_file: bool,
        is_symlink: bool,
        size: u64,
        modified: Option<String>,
        permissions: String,
        owner: String,
        group: String,
    }

    let json_entries: Vec<JsonEntry> = entries.iter().map(|e| JsonEntry {
        name: e.name.clone(),
        path: e.path.to_string_lossy().into_owned(),
        is_dir: e.is_dir,
        is_file: e.is_file,
        is_symlink: e.is_symlink,
        size: e.size,
        modified: e.modified.map(|dt| dt.to_rfc3339()),
        permissions: e.permissions_str(),
        owner: e.owner_name(),
        group: e.group_name(),
    }).collect();

    serde_json::to_string_pretty(&json_entries).unwrap_or_else(|_| "[]".to_string())
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entry: FileEntry,
    pub children: Vec<TreeNode>,
}

pub fn format_tree_node(node: &TreeNode, prefix: &str, is_last: bool, is_root: bool) -> String {
    let mut output = String::new();

    if is_root {
        output.push_str(&colorize_name(&node.entry));
        output.push('\n');
    } else {
        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&colorize_name(&node.entry));
        output.push('\n');
    }

    let next_prefix_addition = if is_root {
        ""
    } else if is_last {
        "    "
    } else {
        "│   "
    };

    let child_prefix = format!("{}{}", prefix, next_prefix_addition);
    let child_count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == child_count - 1;
        output.push_str(&format_tree_node(child, &child_prefix, child_is_last, false));
    }

    output
}
