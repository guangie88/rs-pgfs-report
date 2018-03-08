use failure::{Backtrace, Context, Fail};
use std;
use std::fmt::{self, Display};
use std::path::PathBuf;

// suppress false positives from cargo-clippy
#[cfg_attr(feature = "cargo-clippy", allow(empty_line_after_outer_attr))]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "Default logger initialization error")]
    DefaultLoggerInit,

    #[fail(display = "File I/O error")]
    FileIo,

    #[fail(display = "Initial fluent post check error")]
    FluentInitCheck,

    #[fail(display = "Fluent post from tagged record error")]
    FluentPostTaggedRecord,

    #[fail(display = "Lock file open error")]
    LockFileOpen,

    #[fail(display = "Lock file exclusive lock error")]
    LockFileExclusiveLock,

    #[fail(display = "Cannot connect to Postgres server")]
    PgConnection,

    #[fail(display = "Cannot execute Postgres query to get database sizes")]
    PgGetDbSizes,

    #[fail(display = "Cannot unsecure connection URL")]
    PgUnsecureUrl,

    #[fail(display = "Specialized logger initialization error")]
    SpecializedLoggerInit,

    #[fail(display = "TOML config parse error")]
    TomlConfigParse,
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Debug, Fail)]
#[fail(display = "{{ path: {:?}, inner: {} }}", path, inner)]
pub struct PathError<E>
where
    E: Fail,
{
    path: PathBuf,

    #[cause]
    inner: E,
}

impl<E> PathError<E>
where
    E: Fail,
{
    pub fn new<P>(path: P, inner: E) -> PathError<E>
    where
        P: Into<PathBuf>,
    {
        PathError {
            path: path.into(),
            inner,
        }
    }
}

#[derive(Debug, Fail)]
#[fail(display = "{{ query: {}, inner: {} }}", query, inner)]
pub struct QueryError<E>
where
    E: Fail,
{
    query: String,

    #[cause]
    inner: E,
}

impl<E> QueryError<E>
where
    E: Fail,
{
    pub fn new<Q>(query: Q, inner: E) -> QueryError<E>
    where
        Q: Into<String>,
    {
        QueryError {
            query: query.into(),
            inner,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{{ inner: {}, cause: {:?}, backtrace: {:?} }}",
            self.inner,
            self.cause(),
            self.backtrace()
        )
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use failure::Fail;

    #[cfg_attr(feature = "cargo-clippy", allow(empty_line_after_outer_attr))]
    #[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
    #[fail(display = "Fake error kind")]
    pub struct FakeErrorKind;

    #[derive(Debug, Fail)]
    #[fail(display = "Fake error")]
    struct FakeError;

    #[test]
    fn test_path_error_trait() {
        PathError::new("Fake path", FakeError).context(FakeErrorKind);
    }

    #[test]
    fn test_query_error_trait() {
        QueryError::new("Fake query", FakeError).context(FakeErrorKind);
    }
}
