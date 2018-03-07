#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

#[macro_use]
extern crate failure;
extern crate fruently;
extern crate fs2;
extern crate json_collection;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate postgres;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_humantime;
extern crate simple_logger;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate toml;

mod conf;
mod error;
mod pg;
mod util;

use conf::{ArgConf, Config, FluentdConfig, PostgresConfig};
use error::{ErrorKind, PathError, QueryError, Result};
use failure::ResultExt;
use fruently::fluent::Fluent;
use fruently::forwardable::JsonForwardable;
use fruently::retry_conf::RetryConf;
use json_collection::{Storage, StorageBuilder};
use pg::DbSize;
use postgres::{Connection, TlsMode};
use std::path::Path;
use std::process;
use std::thread;
use structopt::StructOpt;

fn db_sizes_to_storage<C, D>(
    connection_url: C,
    cap: u64,
    db_sizes: D,
) -> Storage
where
    C: AsRef<str>,
    D: Iterator<Item = DbSize>,
{
    let used = db_sizes
        .map(|db_size| db_size.size as u64)
        .sum();

    StorageBuilder::default()
        .path(connection_url.as_ref())
        .capacity(cap)
        .used(used)
        .build()
}

fn create_and_check_fluent(f_conf: &FluentdConfig) -> Result<Fluent<&String>> {
    let fluent_conf = RetryConf::new()
        .max(f_conf.try_count)
        .multiplier(f_conf.multiplier);

    let fluent_conf = match f_conf.store_file_path {
        Some(ref store_file_path) => {
            fluent_conf.store_file(Path::new(store_file_path).to_owned())
        }
        None => fluent_conf,
    };

    let fluent = Fluent::new_with_conf(
        &f_conf.address,
        f_conf.tag.as_str(),
        fluent_conf,
    );

    fluent
        .clone()
        .post("rs-pgfs-report-log-initialization")
        .context(ErrorKind::FluentInitCheck)?;

    Ok(fluent)
}

fn read_config_file<P>(conf_path: P) -> Result<Config>
where
    P: AsRef<Path>,
{
    let conf_path = conf_path.as_ref();

    let config: Config = toml::from_str(&util::read_from_file(conf_path)?)
        .map_err(|e| PathError::new(conf_path, e))
        .context(ErrorKind::TomlConfigParse)?;

    Ok(config)
}

fn create_conn(pg_conf: &PostgresConfig) -> Result<Connection> {
    let conn =
        Connection::connect(pg_conf.connection_url.as_str(), TlsMode::None)
            .map_err(|e| PathError::new(&pg_conf.connection_url, e))
            .context(ErrorKind::PgConnection)?;

    Ok(conn)
}

fn get_db_sizes(conn: &Connection) -> Result<Vec<DbSize>> {
    const DB_SIZES_QUERY: &str =
        "SELECT pg_database.datname AS name, \
         pg_database_size(pg_database.datname) AS size FROM pg_database;";

    let db_size_rows = conn.query(DB_SIZES_QUERY, &[])
        .map_err(|e| QueryError::new(DB_SIZES_QUERY, e))
        .context(ErrorKind::PgGetDbSizes)?;

    let db_sizes: Vec<DbSize> = db_size_rows
        .into_iter()
        .map(|db_size_row| {
            let db: String = db_size_row.get("name");
            let size = db_size_row.get("size");
            DbSize::new(db, size)
        })
        .collect();

    debug!("```\n{:#?}```", db_sizes);

    Ok(db_sizes)
}

fn run_impl(conf: &Config) -> Result<()> {
    let fluent = create_and_check_fluent(&conf.fluentd)?;
    let conn = create_conn(&conf.pg)?;
    let db_sizes = get_db_sizes(&conn)?;

    let storage = db_sizes_to_storage(
        &conf.pg.connection_url,
        conf.system.estimated_cap,
        db_sizes.into_iter(),
    );

    fluent
        .clone()
        .post(&storage)
        .context(ErrorKind::FluentPostTaggedRecord)?;

    Ok(())
}

fn run(conf: &Config) -> Result<()> {
    // to check if the process is already running as another PID
    let _flock = util::lock_file(&conf.general.lock_file)?;

    match conf.general.repeat_delay {
        Some(repeat_delay) => loop {
            print_run_status(&run_impl(conf));
            thread::sleep(repeat_delay)
        },
        None => run_impl(conf),
    }
}

fn init() -> Result<Config> {
    let arg_conf = ArgConf::from_args();
    let conf = read_config_file(&arg_conf.conf)?;

    match conf.general.log_conf_path {
        Some(ref log_conf_path) => {
            log4rs::init_file(log_conf_path, Default::default())
                .map_err(|e| PathError::new(log_conf_path, e))
                .context(ErrorKind::SpecializedLoggerInit)?
        }
        None => simple_logger::init().context(ErrorKind::DefaultLoggerInit)?,
    }

    Ok(conf)
}

fn print_run_status(res: &Result<()>) {
    match *res {
        Ok(_) => info!("Session completed!"),
        Err(ref e) => {
            error!("{}", e);
        }
    }
}

fn main() {
    let conf_res = init();

    if let Err(ref e) = conf_res {
        eprintln!("{}", e);
    }

    let res = conf_res.and_then(|conf| {
        info!("Program started!");
        debug!("```\n{:#?}```", conf);
        run(&conf)
    });

    print_run_status(&res);

    if res.is_err() {
        process::exit(1);
    }
}
