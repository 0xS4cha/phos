use std::path::Path;
use crate::entry::FileEntry;
use crate::filter::{FileFilter, CompositeFilter, GlobFilter, ExtensionFilter, SizeFilter, TypeFilter, ModifiedFilter, HiddenFilter, parse_duration};
use crate::sort::Sorter;
use crate::preset::{Preset, parse_size};
use crate::format::{TreeNode, OutputFormat};

pub struct Engine {
    preset: Preset,
}

impl Engine {
    pub fn new(preset: Preset) -> Self {
        Self { preset }
    }

    pub fn build_filter(&self) -> CompositeFilter {
        let mut composite = CompositeFilter::new();

        if let Some(ref pat) = self.preset.pattern {
            if let Ok(glob_filter) = GlobFilter::new(pat) {
                composite.add(Box::new(glob_filter));
            }
        }

        if let Some(ref exts) = self.preset.extensions {
            composite.add(Box::new(ExtensionFilter::new(exts.clone())));
        }

        let min_size = self.preset.min_size.as_ref().and_then(|s| parse_size(s));
        let max_size = self.preset.max_size.as_ref().and_then(|s| parse_size(s));
        if min_size.is_some() || max_size.is_some() {
            composite.add(Box::new(SizeFilter::new(min_size, max_size)));
        }

        let dirs_only = self.preset.dirs_only.unwrap_or(false);
        let files_only = self.preset.files_only.unwrap_or(false);
        let no_symlinks = self.preset.no_symlinks.unwrap_or(false);
        if dirs_only || files_only || no_symlinks {
            composite.add(Box::new(TypeFilter {
                dirs_only,
                files_only,
                no_symlinks,
            }));
        }

        if let Some(ref within) = self.preset.modified_within {
            if let Some(dur) = parse_duration(within) {
                let cutoff = chrono::Local::now() - dur;
                composite.add(Box::new(ModifiedFilter::new(cutoff, true)));
            }
        }
        if let Some(ref before) = self.preset.modified_before {
            if let Some(dur) = parse_duration(before) {
                let cutoff = chrono::Local::now() - dur;
                composite.add(Box::new(ModifiedFilter::new(cutoff, false)));
            }
        }

        if !self.preset.show_hidden.unwrap_or(false) {
            composite.add(Box::new(HiddenFilter));
        }

        composite
    }

    fn pre_filter_matches(&self, name: &str, is_dir: bool, _is_file: bool, is_symlink: bool) -> bool {
        if !self.preset.show_hidden.unwrap_or(false) && name.starts_with('.') {
            return false;
        }

        if is_dir {
            return true;
        }

        if self.preset.dirs_only.unwrap_or(false) {
            return false;
        }

        if is_symlink && self.preset.no_symlinks.unwrap_or(false) {
            return false;
        }

        if let Some(ref exts) = self.preset.extensions {
            if let Some(ext) = Path::new(name).extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if !exts.iter().any(|e| e.to_lowercase() == ext_str) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref pat) = self.preset.pattern {
            if let Ok(pattern) = glob::Pattern::new(pat) {
                if !pattern.matches(name) {
                    return false;
                }
            }
        }

        true
    }

    pub fn run_flat_stream(&self, dir: &Path) -> std::io::Result<()> {
        let filter = self.build_filter();
        let max_depth = self.preset.max_depth;
        let format = self.preset.output_format.unwrap_or(OutputFormat::Grid);
        let human_readable = self.preset.human_readable.unwrap_or(true);

        let root_meta = std::fs::symlink_metadata(dir)?;
        let root_entry = FileEntry::new(dir.to_path_buf(), Some(root_meta))?;

        if root_entry.is_file || root_entry.is_symlink {
            if filter.matches(&root_entry) {
                let entries = vec![root_entry];
                let output = match format {
                    OutputFormat::Grid => crate::format::format_grid(&entries),
                    OutputFormat::Long => crate::format::format_long(&entries, human_readable),
                    _ => unreachable!(),
                };
                print!("{}", output);
            }
        } else {
            let mut is_first_dir = true;
            self.walk_flat_stream(dir, 0, max_depth, &mut is_first_dir, &filter, format, human_readable)?;
        }

        Ok(())
    }

    fn walk_flat_stream(
        &self,
        dir: &Path,
        current_depth: usize,
        max_depth: Option<usize>,
        is_first_dir: &mut bool,
        filter: &dyn FileFilter,
        format: OutputFormat,
        human_readable: bool,
    ) -> std::io::Result<()> {
        if let Some(d) = max_depth {
            if current_depth > d {
                return Ok(());
            }
        } else if current_depth > 0 {
            return Ok(());
        }

        let mut entries = Vec::new();
        let mut subdirs = Vec::new();

        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry_res in read_dir.flatten() {
                let name = entry_res.file_name();
                let name_str = name.to_string_lossy();
                if let Ok(file_type) = entry_res.file_type() {
                    let is_dir = file_type.is_dir();
                    let is_file = file_type.is_file();
                    let is_symlink = file_type.is_symlink();

                    if !self.pre_filter_matches(&name_str, is_dir, is_file, is_symlink) {
                        if is_dir && max_depth.is_some() {
                            subdirs.push(entry_res.path());
                        }
                        continue;
                    }

                    if let Ok(meta) = entry_res.metadata() {
                        if let Ok(file_entry) = FileEntry::new(entry_res.path(), Some(meta)) {
                            if filter.matches(&file_entry) {
                                entries.push(file_entry.clone());
                            }
                            if file_entry.is_dir && max_depth.is_some() {
                                subdirs.push(file_entry.path);
                            }
                        }
                    }
                }
            }
        }

        let sorter = Sorter {
            field: self.preset.sort_field.unwrap_or(crate::sort::SortField::Name),
            reverse: self.preset.reverse.unwrap_or(false),
            dirs_first: self.preset.dirs_first.unwrap_or(true),
        };
        sorter.sort(&mut entries);

        subdirs.sort_by(|a, b| {
            let name_a = a.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            let name_b = b.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
            name_a.cmp(&name_b)
        });

        let is_recursive = max_depth.is_some();

        if is_recursive {
            if !*is_first_dir {
                println!();
            }
            *is_first_dir = false;
            println!("{}:", dir.display());
        }

        if !entries.is_empty() {
            let output = match format {
                OutputFormat::Grid => crate::format::format_grid(&entries),
                OutputFormat::Long => crate::format::format_long(&entries, human_readable),
                _ => unreachable!(),
            };
            print!("{}", output);
        }

        for subdir in subdirs {
            self.walk_flat_stream(&subdir, current_depth + 1, max_depth, is_first_dir, filter, format, human_readable)?;
        }

        Ok(())
    }

    pub fn run_flat_json(&self, dir: &Path) -> std::io::Result<Vec<FileEntry>> {
        let filter = self.build_filter();
        let max_depth = self.preset.max_depth;
        let mut entries = Vec::new();

        let root_meta = std::fs::symlink_metadata(dir)?;
        let root_entry = FileEntry::new(dir.to_path_buf(), Some(root_meta))?;

        if root_entry.is_file || root_entry.is_symlink {
            if filter.matches(&root_entry) {
                entries.push(root_entry);
            }
        } else {
            self.walk_flat_accumulate(dir, 0, max_depth, &filter, &mut entries)?;
        }

        let sorter = Sorter {
            field: self.preset.sort_field.unwrap_or(crate::sort::SortField::Name),
            reverse: self.preset.reverse.unwrap_or(false),
            dirs_first: self.preset.dirs_first.unwrap_or(true),
        };
        sorter.sort(&mut entries);

        Ok(entries)
    }

    fn walk_flat_accumulate(
        &self,
        dir: &Path,
        current_depth: usize,
        max_depth: Option<usize>,
        filter: &dyn FileFilter,
        acc: &mut Vec<FileEntry>,
    ) -> std::io::Result<()> {
        if let Some(d) = max_depth {
            if current_depth > d {
                return Ok(());
            }
        } else if current_depth > 0 {
            return Ok(());
        }

        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry_res in read_dir.flatten() {
                let name = entry_res.file_name();
                let name_str = name.to_string_lossy();
                if let Ok(file_type) = entry_res.file_type() {
                    let is_dir = file_type.is_dir();
                    let is_file = file_type.is_file();
                    let is_symlink = file_type.is_symlink();

                    if !self.pre_filter_matches(&name_str, is_dir, is_file, is_symlink) {
                        if is_dir && max_depth.is_some() {
                            self.walk_flat_accumulate(&entry_res.path(), current_depth + 1, max_depth, filter, acc)?;
                        }
                        continue;
                    }

                    if let Ok(meta) = entry_res.metadata() {
                        if let Ok(file_entry) = FileEntry::new(entry_res.path(), Some(meta)) {
                            if filter.matches(&file_entry) {
                                acc.push(file_entry.clone());
                            }
                            if file_entry.is_dir && max_depth.is_some() {
                                self.walk_flat_accumulate(&file_entry.path, current_depth + 1, max_depth, filter, acc)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn run_tree(&self, dir: &Path) -> std::io::Result<Option<TreeNode>> {
        let filter = self.build_filter();
        let max_depth = self.preset.max_depth;

        let sorter = Sorter {
            field: self.preset.sort_field.unwrap_or(crate::sort::SortField::Name),
            reverse: self.preset.reverse.unwrap_or(false),
            dirs_first: self.preset.dirs_first.unwrap_or(true),
        };

        let sort_fn = move |entries: &mut [FileEntry]| {
            sorter.sort(entries);
        };

        let root_meta = std::fs::symlink_metadata(dir)?;
        let root_entry = FileEntry::new(dir.to_path_buf(), Some(root_meta))?;

        if root_entry.is_file || root_entry.is_symlink {
            if filter.matches(&root_entry) {
                Ok(Some(TreeNode {
                    entry: root_entry,
                    children: Vec::new(),
                }))
            } else {
                Ok(None)
            }
        } else {
            self.walk_tree_recursive(dir, max_depth, 0, &filter, &sort_fn)
        }
    }

    fn walk_tree_recursive(
        &self,
        dir: &Path,
        max_depth: Option<usize>,
        current_depth: usize,
        filter: &dyn FileFilter,
        sort_fn: &dyn Fn(&mut [FileEntry]),
    ) -> std::io::Result<Option<TreeNode>> {
        let root_meta = std::fs::symlink_metadata(dir)?;
        let root_entry = FileEntry::new(dir.to_path_buf(), Some(root_meta))?;

        if let Some(d) = max_depth {
            if current_depth > d {
                return Ok(None);
            }
        } else if current_depth > 0 {
            return Ok(None);
        }

        let mut children = Vec::new();
        if root_entry.is_dir {
            if let Ok(read_dir) = std::fs::read_dir(dir) {
                let mut all_entries = Vec::new();
                for entry_res in read_dir.flatten() {
                    let name = entry_res.file_name();
                    let name_str = name.to_string_lossy();
                    if let Ok(file_type) = entry_res.file_type() {
                        let is_dir = file_type.is_dir();
                        let is_file = file_type.is_file();
                        let is_symlink = file_type.is_symlink();

                        if !self.pre_filter_matches(&name_str, is_dir, is_file, is_symlink) {
                            continue;
                        }

                        if let Ok(meta) = entry_res.metadata() {
                            if let Ok(file_entry) = FileEntry::new(entry_res.path(), Some(meta)) {
                                all_entries.push(file_entry);
                            }
                        }
                    }
                }

                sort_fn(&mut all_entries);

                for entry in all_entries {
                    if entry.is_dir {
                        if max_depth.is_some() {
                            if let Some(sub_node) = self.walk_tree_recursive(
                                &entry.path,
                                max_depth,
                                current_depth + 1,
                                filter,
                                sort_fn,
                            )? {
                                let matches_self = filter.matches(&entry);
                                let has_children = !sub_node.children.is_empty();
                                if matches_self || has_children {
                                    children.push(sub_node);
                                }
                            }
                        } else {
                            if filter.matches(&entry) {
                                children.push(TreeNode {
                                    entry,
                                    children: Vec::new(),
                                });
                            }
                        }
                    } else {
                        if filter.matches(&entry) {
                            children.push(TreeNode {
                                entry,
                                children: Vec::new(),
                            });
                        }
                    }
                }
            }
        }

        Ok(Some(TreeNode {
            entry: root_entry,
            children,
        }))
    }
}
