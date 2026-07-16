use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct UserCache {
    users: HashMap<u32, String>,
    groups: HashMap<u32, String>,
}

impl UserCache {
    fn new() -> Self {
        let mut users = HashMap::new();
        let mut groups = HashMap::new();

        if let Ok(file) = File::open("/etc/passwd") {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let username = parts[0].to_string();
                    if let Ok(uid) = parts[2].parse::<u32>() {
                        users.insert(uid, username);
                    }
                }
            }
        }

        if let Ok(file) = File::open("/etc/group") {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let groupname = parts[0].to_string();
                    if let Ok(gid) = parts[2].parse::<u32>() {
                        groups.insert(gid, groupname);
                    }
                }
            }
        }

        Self { users, groups }
    }

    pub fn get_user(&self, uid: u32) -> String {
        self.users.get(&uid).cloned().unwrap_or_else(|| uid.to_string())
    }

    pub fn get_group(&self, gid: u32) -> String {
        self.groups.get(&gid).cloned().unwrap_or_else(|| gid.to_string())
    }
}

pub fn get_username(uid: u32) -> String {
    static CACHE: OnceLock<Mutex<UserCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(UserCache::new()));
    cache.lock().unwrap().get_user(uid)
}

pub fn get_groupname(gid: u32) -> String {
    static CACHE: OnceLock<Mutex<UserCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(UserCache::new()));
    cache.lock().unwrap().get_group(gid)
}
