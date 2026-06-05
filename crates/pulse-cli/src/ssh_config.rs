use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SshHost {
    pub name: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    pub identity_file: Option<String>,
}

/// Parse ~/.ssh/config and return discovered hosts.
pub fn parse_ssh_config() -> Vec<SshHost> {
    let path = ssh_config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut hosts = Vec::new();
    let mut current: Option<SshHost> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0].to_lowercase();
        let value = parts[1].trim().to_string();

        match key.as_str() {
            "host" => {
                if let Some(h) = current.take()
                    && !h.hostname.is_empty()
                    && !h.name.contains('*')
                {
                    hosts.push(h);
                }
                current = Some(SshHost {
                    name: value,
                    hostname: String::new(),
                    user: String::new(),
                    port: 22,
                    identity_file: None,
                });
            }
            "hostname" => {
                if let Some(ref mut h) = current {
                    h.hostname = value;
                }
            }
            "user" => {
                if let Some(ref mut h) = current {
                    h.user = value;
                }
            }
            "port" => {
                if let Some(ref mut h) = current {
                    h.port = value.parse().unwrap_or(22);
                }
            }
            "identityfile" => {
                if let Some(ref mut h) = current {
                    h.identity_file = Some(expand_tilde(&value));
                }
            }
            _ => {}
        }
    }

    if let Some(h) = current
        && !h.hostname.is_empty()
        && !h.name.contains('*')
    {
        hosts.push(h);
    }

    hosts
}

/// Find default SSH keys on the system.
pub fn find_default_keys() -> Vec<String> {
    let ssh_dir = dirs_home().join(".ssh");
    let candidates = ["id_ed25519", "id_rsa", "id_ecdsa"];
    candidates.iter().map(|k| ssh_dir.join(k)).filter(|p| p.exists()).map(|p| p.to_string_lossy().to_string()).collect()
}

fn ssh_config_path() -> PathBuf {
    dirs_home().join(".ssh").join("config")
}

fn dirs_home() -> PathBuf {
    directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        let home = dirs_home();
        format!("{}{}", home.display(), &path[1..])
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_config_format() {
        // This test just verifies the parser doesn't panic on real-world data
        let hosts = parse_ssh_config();
        // On the dev machine, there should be some hosts configured
        for h in &hosts {
            assert!(!h.name.is_empty());
            assert!(!h.hostname.is_empty());
        }
    }

    #[test]
    fn test_find_default_keys() {
        let keys = find_default_keys();
        // On most dev machines, at least one key exists
        for k in &keys {
            assert!(std::path::Path::new(k).exists());
        }
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/.ssh/id_ed25519");
        assert!(!expanded.starts_with('~'));
        assert!(expanded.contains(".ssh/id_ed25519"));
    }
}
