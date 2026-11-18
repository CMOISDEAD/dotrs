//! dotrs: a minimalist dotfiles manager.
//!
//! Dotfiles are plain copies kept in a dots directory (a git repository).
//! Nothing is symlinked: `apply` copies from the repository into HOME,
//! `sync` copies local changes back, and the SHA-256 recorded at the last
//! sync tells which side changed.

pub mod backup;
pub mod commands;
pub mod context;
pub mod error;
pub mod ignore;
pub mod output;
pub mod paths;
pub mod scan;
pub mod state;

pub use context::Context;
pub use error::{Error, Result};
