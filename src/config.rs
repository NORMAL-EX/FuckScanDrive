use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProcessRule {
    pub process_name: String,
    pub blocked_drives: BlockedDrives,
}

#[derive(Debug, Clone)]
pub enum BlockedDrives {
    All,
    Specific(HashSet<String>),
}

impl BlockedDrives {
    pub fn is_blocked(&self, drive: &str) -> bool {
        match self {
            BlockedDrives::All => true,
            BlockedDrives::Specific(drives) => {
                let normalized = drive.to_uppercase().chars().next().unwrap_or('\0');
                drives.iter().any(|d| {
                    let d_char = d.to_uppercase().chars().next().unwrap_or('\0');
                    d_char == normalized
                })
            }
        }
    }
}

pub struct Config {
    pub rules: Vec<ProcessRule>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let mut rules = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let process_name = parts[0].to_lowercase();

            if parts.len() == 1 {
                return Err(format!(
                    "Line {}: Missing drive specification for process '{}'",
                    line_num + 1,
                    process_name
                ));
            }

            let blocked_drives = if parts[1].eq_ignore_ascii_case("All") {
                BlockedDrives::All
            } else {
                let mut drives = HashSet::new();
                for drive_spec in &parts[1..] {
                    let drive = drive_spec.trim_end_matches(':').to_uppercase();
                    if drive.len() == 1 && drive.chars().next().unwrap().is_ascii_alphabetic() {
                        drives.insert(drive);
                    } else {
                        return Err(format!(
                            "Line {}: Invalid drive specification '{}'. Expected format: 'C:' or 'C'",
                            line_num + 1,
                            drive_spec
                        ));
                    }
                }

                if drives.is_empty() {
                    return Err(format!(
                        "Line {}: No valid drives specified for process '{}'",
                        line_num + 1,
                        process_name
                    ));
                }

                BlockedDrives::Specific(drives)
            };

            rules.push(ProcessRule {
                process_name,
                blocked_drives,
            });
        }

        if rules.is_empty() {
            return Err("Config file contains no valid rules".to_string());
        }

        Ok(Config { rules })
    }

    pub fn find_rule(&self, process_name: &str) -> Option<&ProcessRule> {
        let normalized_name = process_name.to_lowercase();
        self.rules.iter().find(|r| r.process_name == normalized_name)
    }

    pub fn reload<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let new_config = Config::load(path)?;
        self.rules = new_config.rules;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_drives_all() {
        let blocked = BlockedDrives::All;
        assert!(blocked.is_blocked("C:"));
        assert!(blocked.is_blocked("D:"));
        assert!(blocked.is_blocked("Z:"));
    }

    #[test]
    fn test_blocked_drives_specific() {
        let mut drives = HashSet::new();
        drives.insert("C".to_string());
        drives.insert("D".to_string());
        let blocked = BlockedDrives::Specific(drives);

        assert!(blocked.is_blocked("C:"));
        assert!(blocked.is_blocked("D:"));
        assert!(!blocked.is_blocked("E:"));
    }

    #[test]
    fn test_config_parse_all() {
        let content = "test.exe All";
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_config_all.ini");
        std::fs::write(&config_path, content).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].process_name, "test.exe");

        std::fs::remove_file(config_path).ok();
    }

    #[test]
    fn test_config_parse_specific() {
        let content = "test.exe C: D:";
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_config_specific.ini");
        std::fs::write(&config_path, content).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.rules.len(), 1);

        std::fs::remove_file(config_path).ok();
    }

    #[test]
    fn test_config_multiple_rules() {
        let content = "test1.exe All\ntest2.exe C: D:\ntest3.exe E:";
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_config_multi.ini");
        std::fs::write(&config_path, content).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.rules.len(), 3);

        std::fs::remove_file(config_path).ok();
    }
}
