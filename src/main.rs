//! otd-convert - CLI tool to convert OTD files to CNI format.

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use otd_convert_rs::{convert_otd_to_cni, parse_otd_file, validate_schemas};

/// Convert OTD files to CNI format for Intermac glass cutting machines.
#[derive(Parser, Debug)]
#[command(name = "otd-convert")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input OTD/OTX file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output CNI file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Machine number (100-199 for cutting tables)
    #[arg(short, long, default_value = "130")]
    machine: u16,

    /// Validate only, don't generate output
    #[arg(long)]
    validate: bool,

    /// Output debug information as JSON
    #[arg(long)]
    debug: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Validate machine number
    if args.machine < 100 || args.machine >= 200 {
        warn!(
            "Machine number {} is outside the cutting table range (100-199)",
            args.machine
        );
    }

    info!("Processing: {}", args.input.display());

    // Parse the input file
    let schemas = parse_otd_file(&args.input)
        .with_context(|| format!("Failed to parse {}", args.input.display()))?;

    info!("Parsed {} pattern(s)", schemas.len());

    // Validate
    let validation = validate_schemas(&schemas)?;

    for warning in &validation.warnings {
        warn!("{}", warning);
    }

    for err in &validation.errors {
        error!("{}", err);
    }

    if !validation.passed {
        anyhow::bail!("Validation failed");
    }

    // Debug output
    if args.debug {
        let json = serde_json::to_string_pretty(&schemas)?;
        println!("{}", json);
        return Ok(());
    }

    // Validate-only mode
    if args.validate {
        info!("Validation passed");
        return Ok(());
    }

    // Generate output
    let cni = convert_otd_to_cni(&args.input, args.machine)?;

    // Write output
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("cni");
        path
    });

    std::fs::write(&output_path, &cni)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    info!("Generated: {}", output_path.display());

    Ok(())
}
