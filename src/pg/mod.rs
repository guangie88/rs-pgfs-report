#[derive(Debug)]
pub struct DbSize {
    pub name: String,
    pub size: i64,
}

impl DbSize {
    pub fn new<S>(name: S, size: i64) -> DbSize
    where
        S: Into<String>,
    {
        DbSize {
            name: name.into(),
            size,
        }
    }
}
