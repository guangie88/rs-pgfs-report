use mega_coll::conf::{app, fluentd, pg};

#[derive(StructOpt, Debug)]
#[structopt(name = "rs-pgfs-report-conf",
            about = "Configuration for Rust pgfs-report")]
pub struct ArgConfig {
    #[structopt(short = "c", long = "conf",
                default_value = "config/rs-pgfs-report.toml",
                help = "Configuration file path")]
    pub conf: String,
}

impl app::ArgConf for ArgConfig {
    fn conf(&self) -> &str {
        &self.conf
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub general: app::Config,
    pub fluentd: fluentd::Config,
    pub pg: pg::Config,
}

impl app::Conf for Config {
    fn general(&self) -> &app::Config {
        &self.general
    }
}
