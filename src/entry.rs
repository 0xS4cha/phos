use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use crate::user_cache::{get_username, get_groupname};

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

impl FileEntry {
    pub fn new(path: PathBuf, metadata: Option<fs::Metadata>) -> std::io::Result<Self> {
        let meta = match metadata {
            Some(m) => m,
            None => fs::symlink_metadata(&path)?,
        };

        let file_type = meta.file_type();
        let name = path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string());

        let modified = meta.modified().ok()
            .map(|t| DateTime::<Local>::from(t));

        #[cfg(unix)]
        let (mode, uid, gid) = {
            use std::os::unix::fs::MetadataExt;
            (Some(meta.mode()), Some(meta.uid()), Some(meta.gid()))
        };

        #[cfg(not(unix))]
        let (mode, uid, gid) = (None, None, None);

        Ok(Self {
            path,
            name,
            is_dir: file_type.is_dir(),
            is_file: file_type.is_file(),
            is_symlink: file_type.is_symlink(),
            size: meta.len(),
            modified,
            mode,
            uid,
            gid,
        })
    }

    pub fn permissions_str(&self) -> String {
        let mode = match self.mode {
            Some(m) => m,
            None => return "---------".to_string(),
        };

        let mut s = String::with_capacity(9);

        s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
        s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
        s.push(if mode & 0o100 != 0 { 'x' } else { '-' });

        s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
        s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
        s.push(if mode & 0o010 != 0 { 'x' } else { '-' });

        s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
        s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
        s.push(if mode & 0o001 != 0 { 'x' } else { '-' });

        s
    }

    pub fn file_type_char(&self) -> char {
        if self.is_dir {
            'd'
        } else if self.is_symlink {
            'l'
        } else {
            '-'
        }
    }

    pub fn owner_name(&self) -> String {
        self.uid.map(get_username).unwrap_or_else(|| "-".to_string())
    }

    pub fn group_name(&self) -> String {
        self.gid.map(get_groupname).unwrap_or_else(|| "-".to_string())
    }
}
