use serde_humantime;
use std::time::Duration;

#[derive(StructOpt, Debug)]
#[structopt(name = "rs-pgfs-report-conf",
            about = "Configuration for Rust pgfs-report")]
pub struct ArgConf {
    #[structopt(short = "c", long = "conf",
                default_value = "config/rs-pgfs-report.toml",
                help = "Configuration file path")]
    pub conf: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub fluentd: FluentdConfig,
    pub general: GeneralConfig,
    pub pg: PostgresConfig,
    pub system: SystemConfig,
}

#[derive(Deserialize, Debug)]
pub struct FluentdConfig {
    pub address: String,
    pub tag: String,
    pub try_count: u64,
    pub multiplier: f64,
    pub store_file_path: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GeneralConfig {
    pub log_conf_path: Option<String>,
    pub lock_file: String,
    #[serde(with = "serde_humantime")]
    pub repeat_delay: Option<Duration>,
}

#[derive(Deserialize, Debug)]
pub struct PostgresConfig {
    pub connection_url: String,
}

#[derive(Deserialize, Debug)]
pub struct SystemConfig {
    pub estimated_cap: u64,
}
