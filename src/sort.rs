use crate::entry::FileEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortField {
    Name,
    Size,
    Time,
    Extension,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sorter {
    pub field: SortField,
    pub reverse: bool,
    pub dirs_first: bool,
}

impl Default for Sorter {
    fn default() -> Self {
        Self {
            field: SortField::Name,
            reverse: false,
            dirs_first: true,
        }
    }
}

impl Sorter {
    pub fn sort(&self, entries: &mut [FileEntry]) {
        entries.sort_by(|a, b| {
            if self.dirs_first && a.is_dir != b.is_dir {
                if a.is_dir {
                    return std::cmp::Ordering::Less;
                } else {
                    return std::cmp::Ordering::Greater;
                }
            }

            let ord = match self.field {
                SortField::Name => {
                    let name_a_lower = a.name.to_lowercase();
                    let name_b_lower = b.name.to_lowercase();
                    let cmp = name_a_lower.cmp(&name_b_lower);
                    if cmp == std::cmp::Ordering::Equal {
                        a.name.cmp(&b.name)
                    } else {
                        cmp
                    }
                }
                SortField::Size => a.size.cmp(&b.size),
                SortField::Time => {
                    match (a.modified, b.modified) {
                        (Some(ta), Some(tb)) => ta.cmp(&tb),
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                SortField::Extension => {
                    let ext_a = a.path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let ext_b = b.path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let cmp = ext_a.cmp(&ext_b);
                    if cmp == std::cmp::Ordering::Equal {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        cmp
                    }
                }
            };

            if self.reverse {
                ord.reverse()
            } else {
                ord
            }
        });
    }
}
