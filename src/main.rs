use std::time::Duration;

use anyhow::*;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use flexi_logger::{colored_default_format, detailed_format, Logger, LoggerHandle, WriteMode};
use human_panic::setup_panic;
use log::{debug, warn, Level};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Import(ImportParams),
    DeleteSome(run_delete::Params),
}

#[derive(Args)]
pub struct ImportParams {
    file: std::path::PathBuf,
}

fn main() -> Result<()> {
    setup_panic!();
    if let Err(env_err) = dotenvy::dotenv() {
        if env_err.not_found() {
            warn!("No `.env` file found (recursively). You usually want to have one.")
        } else {
            return Err(env_err).with_context(|| "Failed to load `.env` file");
        }
    }
    let cli = Cli::parse();
    let logger = configure_log_from(&cli)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .with_context(|| "Failed to start Tokio runtime")?;
    let _guard = runtime.enter();

    let res = do_start(cli);

    debug!("Waiting up to 15 seconds for remaining tasks to finish");
    runtime.shutdown_timeout(Duration::from_secs(15));

    // Important with non-direct write mode
    // Handle needs to be kept alive until end of program
    logger.flush();

    res
}

fn configure_log_from(params: &Cli) -> Result<LoggerHandle> {
    // log_level() returns None iff verbosity < 0, i.e. being most quiet seems reasonable
    let cli_level = params.verbose.log_level().unwrap_or(Level::Error);

    let log_builder = Logger::try_with_env_or_str(cli_level.to_string())
        .context("Failed to parse logger spec from env RUST_LOG or cli level")?
        .write_mode(WriteMode::Async)
        .format_for_stdout(colored_default_format)
        .format_for_files(detailed_format);

    log_builder
        .start()
        .context("Failed to start logger handle w/o specfile")
}

fn do_start(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Import(it) => import::run(it),
        Commands::DeleteSome(it) => run_delete::run(it),
    }
}

mod import;
mod run_delete;
