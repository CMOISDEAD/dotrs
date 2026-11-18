//! Tiny terminal output helpers. Colors are only emitted when the stream is a
//! terminal and `NO_COLOR` is not set (https://no-color.org).

use std::{
    env,
    fmt::Display,
    io::{self, IsTerminal},
    sync::OnceLock,
};

pub const RED: &str = "31";
pub const GREEN: &str = "32";
pub const YELLOW: &str = "33";
pub const BLUE: &str = "34";
pub const MAGENTA: &str = "35";
pub const CYAN: &str = "36";
pub const DIM: &str = "2";

fn no_color() -> bool {
    env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

pub fn stdout_is_tty() -> bool {
    static TTY: OnceLock<bool> = OnceLock::new();
    *TTY.get_or_init(|| io::stdout().is_terminal())
}

pub fn color_stdout() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| stdout_is_tty() && !no_color())
}

fn color_stderr() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| io::stderr().is_terminal() && !no_color())
}

fn paint_if(on: bool, code: &str, text: impl Display) -> String {
    if on {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Colorize text written to stdout.
pub fn paint(code: &str, text: impl Display) -> String {
    paint_if(color_stdout(), code, text)
}

pub fn error(message: impl Display) {
    eprintln!("{} {message}", paint_if(color_stderr(), RED, "error:"));
}

pub fn warn(message: impl Display) {
    eprintln!("{} {message}", paint_if(color_stderr(), YELLOW, "warning:"));
}
