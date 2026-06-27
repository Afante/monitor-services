use garde::Validate;
use std::{env, io::Read};
use std::collections::HashMap;
use serde_json::{Value as JSValue, Map as JSMap};
use serde::Deserialize;
use std::path::PathBuf;
use glob::glob;
use regex::Regex;
use std::sync::OnceLock;
use std::fs::File;
use garde::Error as GError;
use std::net::IpAddr;
use url::Url;
use crate::log::*;

static _REGEX_DEF: OnceLock<RegexDef> = OnceLock::new();

fn validate_ip_or_hostname(value: &Option<String>, _context: &()) -> Result<(), GError> {
    if let Some(value) = value {
        // Check if it's a valid IP
        if value.parse::<IpAddr>().is_ok() {
            return Ok(());
        }
        
        // Check if it's a valid hostname
        let is_valid_hostname = value
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || c == '-'));
        
        if is_valid_hostname {
            Ok(())
        } else {
            Err(GError::new("must be a valid IP address or hostname"))
        }
    }
    else {
        Ok(())
    }
}


fn validate_url_or_empty(value: &String, _context: &()) -> Result<(), GError> {
    if value.len() == 0 {
        Ok(())
    }
    else {
        match Url::parse(value.as_str()) {
            Ok(_) => Ok(()),
            Err(e) => Err(GError::new(e.to_string().as_str()))
        }
    }
}

pub struct CmdArgs {
    pub cfg_dir: String
}

impl CmdArgs {
    pub fn parse() -> Result<Self, String> {
        let args : Vec<String> = env::args().collect();
        if args.len() > 1 {
            Ok(Self {
                cfg_dir: args[1].clone()
            })
        }
        else {
            Err(String::from("Not enough arguments."))
        }
    }
}

pub fn get_regex_def() -> &'static RegexDef {
    _REGEX_DEF.get_or_init(|| RegexDef::new())
}

pub struct RegexDef {
    pub env_subs: Regex
}

impl RegexDef {
    pub fn new() -> Self {
        Self {
            env_subs: Regex::new(r#"\$\{([a-zA-Z0-9_-]+)\}"#).unwrap()
        }
    }
}

pub fn resolve_string(text: &String) -> String {
    let regex_def = get_regex_def();
    let result = regex_def.env_subs.replace_all(text.as_str(), |caps: &regex::Captures| {
        let env_name = &caps[1];
        let env_value = match env::var(env_name) {
            Ok(v) => v,
            _ => {
                panic!("Cannot get environment variable: {}", env_name);
            }
        };
        env_value
    });
    result.into_owned()
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "kebab-case")]
pub struct MonitorTarget {
    #[garde(length(min = 1))]
    pub kind: String,
    #[serde(default = "String::new")]
    #[garde(custom(validate_url_or_empty))]
    pub url: String,
    #[serde(default = "defaults::method")]
    #[garde(skip)]
    pub method: String,
    #[serde(default = "defaults::none")]
    #[garde(skip)]
    pub body: Option<String>,
    #[garde(skip)]
    pub check_interval_secs: Option<i32>,
    #[garde(skip)]
    pub expect_status_code: Option<u16>,
    #[garde(skip)]
    pub expect_match: Option<String>,
    #[garde(skip)]
    pub expect_unmatch: Option<String>,
    #[serde(default = "String::new")]
    #[garde(skip)]
    pub custom_check_cmd: String,
    #[serde(default = "String::new")]
    #[garde(skip)]
    pub recovery_cmd: String,
    #[serde(default = "defaults::none")]
    #[garde(skip)]
    pub connect_timeout: Option<u64>,
    #[serde(default = "defaults::none")]
    #[garde(skip)]
    pub timeout: Option<u64>,
    #[serde(default = "defaults::none")]
    #[garde(skip)]
    pub read_timeout: Option<u64>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "kebab-case")]
pub struct CommonSettings {
    #[garde(email)]
    pub report_to: Option<String>,

    #[garde(email)]
    pub report_from: Option<String>,

    #[garde(custom(validate_ip_or_hostname))]
    pub smtp_host: Option<String>,

    #[garde(range(min = 1))]
    pub smtp_port: Option<u16>,

    #[garde(length(min = 1))]
    pub smtp_user: Option<String>,

    #[garde(length(min = 1))]
    pub smtp_pass: Option<String>,

    #[serde(default)]
    #[garde(skip)]
    pub smtp_starttls: bool,

    #[serde(default)]
    #[garde(skip)]
    pub smtp_no_verify_hostname: bool,

    #[serde(default)]
    #[garde(skip)]
    pub smtp_no_check_certificate: bool,

    #[serde(default = "defaults::check_interval_secs")]
    #[garde(skip)]
    pub check_interval_secs: i32,

    #[serde(default = "defaults::min_log_level")]
    #[garde(skip)]
    pub min_log_level: LogLevel,

    #[serde(default = "defaults::connect_timeout")]
    #[garde(skip)]
    pub connect_timeout: u64,

    #[serde(default = "defaults::read_timeout")]
    #[garde(skip)]
    pub read_timeout: u64,

    #[serde(default = "defaults::timeout")]
    #[garde(skip)]
    pub timeout: u64,
}

mod defaults {
    use crate::log::LogLevel;

    pub fn check_interval_secs() -> i32 { 30 }
    pub fn min_log_level() -> LogLevel {LogLevel::Info}
    pub fn method() -> String {"GET".to_owned()}
    pub fn none<T>() -> Option<T> {None}
    pub fn connect_timeout() -> u64 {30}
    pub fn timeout() -> u64 {30}
    pub fn read_timeout() -> u64 {30}
}

#[derive(Debug, Deserialize)]
pub struct ProgramSettings {
    #[serde(flatten)]
    pub monitor_targets: HashMap<String, MonitorTarget>,

    #[serde(rename = "_common")]
    pub common_settings: CommonSettings
}

pub fn load_settings_from_dir(cfg_dir: &String) -> Result<ProgramSettings, String> {
    let mut path_buf = PathBuf::new();
    path_buf.push(cfg_dir);
    path_buf.push("*.yml");
    let path_pattern = path_buf.as_path().to_str().unwrap();

    let mut monitor_targets = HashMap::<String, MonitorTarget>::new();
    let mut cs_value = JSMap::new();
    for path_rs in glob(path_pattern).expect("Failed to glob.") {
        let path = match path_rs {
            Ok(p) => p,
            Err(err) => return Err(err.to_string())
        };
        let mut file = match File::open(path.to_str().unwrap()) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Failed to open {} for read.", err.to_string());
                continue
            }
        };
        let mut file_contents = String::new();
        file.read_to_string(&mut file_contents).unwrap();
        let file_contents = resolve_string(&file_contents);
        let config: Result<JSValue, _> = serde_saphyr::from_str(file_contents.as_str());
        let config = match config {
            Ok(cfg) => cfg,
            Err(err) => {
                return Err(format!("Failed to load {}: {}", path.to_str().unwrap(), err.to_string()));
            }
        };
        match config {
            JSValue::Object(obj) => {
                for (key, value) in obj.iter() {
                    match key.as_str() {
                        "_common" => {
                            if let JSValue::Object(map) = value {
                                for (k, v) in map {
                                    cs_value.insert(k.clone(), v.clone());
                                }
                            }
                        },
                        setting_name => {
                            let monitor_target: MonitorTarget = serde_json::from_value(value.clone()).unwrap();
                            monitor_targets.insert(String::from(setting_name), monitor_target);
                        }
                    }
                }
            },
            JSValue::Null => {
                eprintln!("{} is empty.", path.to_str().unwrap());
            },
            _ => {
                eprintln!("Unsupported data type at root level in {}", path.to_str().unwrap());
            }
        };
    }

    Ok(ProgramSettings {
        monitor_targets,
        common_settings: serde_json::from_value(JSValue::Object(cs_value)).unwrap()
    })
}
