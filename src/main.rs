//! This crate implements Summer, a CLI tool to summarize the contents of a
//! directory.
//!
//! The sources are split in three submodules:
//!
//! * `config` defines all types to load configuration settings from a YAML file.
//! * `display` takes a summary and print it to the standard output.
//! * `summarizer` reads the contents of a directory, and generates a summary
//!   following the columns defined in the configuration.

mod config;
mod display;
mod summarizer;

#[cfg(all(target_os = "linux", test))]
#[path = "tests/ui/mod.rs"]
mod tests_ui;

use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::process::exit;

xflags::xflags! {
    /// Summarize the contents of a directory.
    cmd summer
        /// Directory to summarize [default: current directory].
        optional path: PathBuf
    {
        /// Path for the configuration file.
        optional -c, --config config: PathBuf

        /// Dump the active configuration.
        optional -D, --dump-config

        /// Prints version information.
        optional -V, --version

        /// Print help information.
        optional -h, --help
    }
}

type AnyError = Box<dyn std::error::Error>;

impl Summer {
    fn run(&self) -> Result<(), AnyError> {
        if self.help {
            print!("{}", Self::HELP);
            return Ok(());
        }

        if self.version {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return Ok(());
        }

        let config = self.load_config()?;

        let stdout_handle = io::stdout();
        let output = BufWriter::new(stdout_handle.lock());

        if self.dump_config {
            serde_yaml::to_writer(output, &config)?;
            return Ok(());
        }

        let path = self.path.as_deref().unwrap_or_else(|| Path::new("."));
        let screen = summarizer::process(path, &config)?;

        display::print(output, screen, &config)?;

        Ok(())
    }

    fn load_config(&self) -> Result<config::Root, config::LoaderError> {
        if let Some(cp) = &self.config {
            return config::load(cp);
        }

        // Path of the default configuration file.

        let config_dir = match dirs::config_dir() {
            Some(d) => d,
            None => {
                eprintln!("Can't get path for the default configuration file.");
                todo!("use a default configuration")
            }
        };

        let path = config_dir.join("summer").join("config.yaml");

        // If the file does not exist, use the default configuration.
        if path.exists() {
            config::load(path)
        } else {
            Ok(config::Root::default())
        }
    }
}

fn main() {
    let summer = match Summer::from_env() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to parser arguments: {}", e);
            exit(1);
        }
    };

    if let Err(e) = summer.run() {
        eprintln!("{}", e);
        exit(1);
    }
}
