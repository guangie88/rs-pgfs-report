#![cfg_attr(feature = "cargo-clippy", deny(warnings))]

extern crate failure;
extern crate fruently;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate mega_coll;
extern crate postgres;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;

mod conf;
mod pg;

use conf::{ArgConfig, Config};
use failure::ResultExt;
use fruently::forwardable::JsonForwardable;
use mega_coll::conf::pg::TlsModeNative;
use mega_coll::error::{ErrorKind, Result};
use mega_coll::error::custom::{PathError, QueryError};
use mega_coll::json::{Storage, StorageBuilder};
use mega_coll::util::app::{create_and_check_fluent, init_config,
                           print_run_status};
use mega_coll::util::fs::lock_file;
use pg::DbSize;
use postgres::{Connection, TlsMode};
use regex::Regex;
use std::borrow::Cow;
use std::process;
use std::thread;

fn mask_url(url: &str) -> Cow<str> {
    lazy_static! {
        static ref REP_RE: Regex = Regex::new(r"//\S*@").unwrap();
    }

    // e.g. postgresql://user:pw@localhost:5432
    REP_RE.replace(url, "//")
}

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
        .path(mask_url(connection_url.as_ref()))
        .capacity(cap)
        .used(used)
        .build()
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
    let fluent = create_and_check_fluent(
        &conf.fluentd,
        "rs-pgfs-report-log-initialization",
    )?;

    let pg_conf = &conf.pg;

    let conn = match pg_conf.tls_mode {
        TlsModeNative::None => {
            Connection::connect(pg_conf.connection_url.as_str(), TlsMode::None)
                .map_err(|e| PathError::new(&pg_conf.connection_url, e))
                .context(ErrorKind::PgConnection)?
        }

        TlsModeNative::Prefer(ref tls) | TlsModeNative::Require(ref tls) => {
            Connection::connect(
                pg_conf.connection_url.as_str(),
                TlsMode::Prefer(tls),
            ).map_err(|e| PathError::new(&pg_conf.connection_url, e))
                .context(ErrorKind::PgConnection)?
        }
    };

    let db_sizes = get_db_sizes(&conn)?;

    let storage = db_sizes_to_storage(
        &conf.pg.connection_url,
        conf.pg.estimated_cap,
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
    let _flock = lock_file(&conf.general.lock_file)?;

    match conf.general.repeat_delay {
        Some(repeat_delay) => loop {
            print_run_status(&run_impl(conf), "Session completed!");
            thread::sleep(repeat_delay)
        },
        None => run_impl(conf),
    }
}

fn main() {
    let conf_res = init_config::<ArgConfig, Config, ErrorKind>();

    if let Err(ref e) = conf_res {
        eprintln!("{}", e);
    }

    let res = conf_res.and_then(|conf| {
        info!("Program started!");
        debug!("```\n{:#?}```", conf);
        run(&conf)
    });

    print_run_status(&res, "Program completed!");

    if res.is_err() {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_url_1() {
        let masked = mask_url("postgresql://user:pw@localhost:5432");
        assert!(masked == "postgresql://localhost:5432");
    }

    #[test]
    fn test_mask_url_2() {
        let masked = mask_url("postgresql://user@localhost:5432");
        assert!(masked == "postgresql://localhost:5432");
    }

    #[test]
    fn test_mask_url_3() {
        let masked = mask_url("postgresql://user:_@localhost:5432");
        assert!(masked == "postgresql://localhost:5432");
    }

    #[test]
    fn test_mask_url_4() {
        let masked = mask_url("postgresql://user:@localhost:5432");
        assert!(masked == "postgresql://localhost:5432");
    }

    #[test]
    fn test_mask_url_5() {
        let masked = mask_url("postgresql://localhost:5432");
        assert!(masked == "postgresql://localhost:5432");
    }

    #[test]
    fn test_mask_url_6() {
        let masked = mask_url("postgresql://localhost");
        assert!(masked == "postgresql://localhost");
    }

    #[test]
    fn test_mask_url_7() {
        let masked = mask_url("localhost");
        assert!(masked == "localhost");
    }
}
