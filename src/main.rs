mod config;
mod error;
mod run;
pub mod util;

use clap::{Parser, Subcommand};
use config::Config;
pub use error::{Error, Reportable};
use run::{cargo, sass, wasm_pack, Html};
use std::env;

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Build artifacts in release mode, with optimizations
    #[arg(short, long)]
    release: bool,

    /// Build for client side rendering. Useful during development due to faster compile times.
    #[arg(long)]
    csr: bool,

    /// Verbosity (none: errors & warnings, -v: verbose, --vv: very verbose, --vvv: output everything)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Subcommand, PartialEq)]
enum Commands {
    /// Adds a default leptos.toml file to current directory
    Init,
    /// Compile the client and server
    Build,
    /// Remove the target directories (in app, client and server)
    Clean,
    /// Run the cargo tests for app, client and server
    Test,
    /// Run the cargo update for app, client and server
    Update,
    /// Run the `ssr` packaged server
    Run,
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    // when running as cargo leptos, the second argument is "leptos" which
    // clap doesn't expect
    if args.get(1).map(|a| a == "leptos").unwrap_or(false) {
        args.remove(1);
    }

    let args = Cli::parse_from(&args);

    util::setup_logging(args.verbose);

    if let Err(e) = try_main(args) {
        log::error!("{e}")
    }
}

fn try_main(args: Cli) -> Result<(), Reportable> {
    if args.command == Commands::Init {
        return config::save_default_file();
    }
    let config = config::read(&args)?;

    match args.command {
        Commands::Init => panic!(),
        Commands::Build => build(&config),
        Commands::Run => {
            build(&config)?;
            cargo::run("run", &config.server_path, &config)?;
            Ok(())
        }
        Commands::Test => {
            cargo::run("test", &config.app_path, &config)?;
            cargo::run("test", &config.client_path, &config)?;
            cargo::run("test", &config.server_path, &config)
        }
        Commands::Clean => {
            cargo::run("clean", &config.app_path, &config)?;
            cargo::run("clean", &config.client_path, &config)?;
            cargo::run("clean", &config.server_path, &config)?;
            util::rm_dir("target")
        }
        Commands::Update => {
            cargo::run("update", &config.app_path, &config)?;
            cargo::run("update", &config.client_path, &config)?;
            cargo::run("update", &config.server_path, &config)
        }
    }
}

fn build(config: &Config) -> Result<(), Reportable> {
    util::rm_dir("target/site")?;

    cargo::run("build", &config.server_path, &config)?;
    sass::run(&config)?;

    let html = Html::read(&config.index_path)?;

    if config.csr {
        wasm_pack::run("build", &config.app_path, &config)?;
        html.generate_html()?;
    } else {
        wasm_pack::run("build", &config.client_path, &config)?;
        html.generate_rust(&config)?;
    }
    Ok(())
}