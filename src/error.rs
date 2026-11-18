use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    /// an I/O error with a description of what was being done
    Io {
        context: String,
        source: io::Error,
    },
    Msg(String),
    /// some files failed; each failure was already reported
    Failed(usize),
    /// not an error, just a non-zero exit status (e.g. `status --exit-code`)
    Exit(u8),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { context, source } => write!(f, "{context}: {source}"),
            Error::Msg(m) => f.write_str(m),
            Error::Failed(1) => f.write_str("1 file failed"),
            Error::Failed(n) => write!(f, "{n} files failed"),
            Error::Exit(_) => Ok(()),
        }
    }
}

impl std::error::Error for Error {}

pub fn msg(m: impl Into<String>) -> Error {
    Error::Msg(m.into())
}

/// Attach context to I/O errors: `fs::copy(a, b).context(|| format!("cannot copy {a}"))?`
pub trait IoContext<T> {
    fn context(self, what: impl FnOnce() -> String) -> Result<T>;
}

impl<T> IoContext<T> for io::Result<T> {
    fn context(self, what: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|source| Error::Io {
            context: what(),
            source,
        })
    }
}
