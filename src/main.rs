mod error;
mod ignore;
mod methods;
mod output;
mod state;
mod utils;

use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand};

use crate::{error::Error, methods::Transfer};

/// Minimalist dotfiles manager: plain copies, no symlinks, no magic.
#[derive(Parser, Debug)]
#[command(name = "dotrs", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Create the dots directory, its .dotrsignore and a git repository
    Init,
    /// Copy files or directories from HOME into the dots directory
    Add {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Stop managing files (deletes them from the dots directory, never from HOME)
    Remove {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// List managed files
    List,
    /// Show the state of every managed file
    Status {
        /// Exit with status 1 if any file is not clean
        #[arg(long)]
        exit_code: bool,
    },
    /// Show differences between the dots directory (-) and HOME (+)
    Diff { paths: Vec<PathBuf> },
    /// Copy dotfiles into HOME (missing files and files updated in the repository)
    Apply(TransferArgs),
    /// Copy files changed in HOME back into the dots directory
    Sync(TransferArgs),
    /// Restore the latest backup of files in HOME (run again to undo)
    Restore {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
}

#[derive(Args, Debug)]
struct TransferArgs {
    /// Only these files or directories (default: all)
    paths: Vec<PathBuf>,
    /// Show what would be done without changing anything
    #[arg(short = 'n', long)]
    dry_run: bool,
    /// Overwrite files even if they changed on the other side
    #[arg(short, long)]
    force: bool,
}

impl TransferArgs {
    fn transfer(&self) -> Transfer<'_> {
        Transfer {
            paths: &self.paths,
            dry_run: self.dry_run,
            force: self.force,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match &cli.command {
        Cmd::Init => methods::init(),
        Cmd::Add { paths } => methods::add(paths),
        Cmd::Remove { paths } => methods::remove(paths),
        Cmd::List => methods::list(),
        Cmd::Status { exit_code } => methods::status(*exit_code),
        Cmd::Diff { paths } => methods::diff(paths),
        Cmd::Apply(args) => methods::apply(args.transfer()),
        Cmd::Sync(args) => methods::sync(args.transfer()),
        Cmd::Restore { paths } => methods::restore(paths),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Exit(code)) => ExitCode::from(code),
        // a single failure was already reported, no need for a summary
        Err(Error::Failed(1)) => ExitCode::FAILURE,
        Err(e) => {
            output::error(e);
            ExitCode::FAILURE
        }
    }
}
