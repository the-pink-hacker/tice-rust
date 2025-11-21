use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Args, Clone)]
pub struct CliFontPackCommand {
    /// The fontpack defintion file
    definition: PathBuf,
    /// The folder to output final asset
    output: PathBuf,
}

#[derive(Debug, Args, Clone)]
pub struct CliSpriteCommand {
    /// The sprite definition file
    definition: PathBuf,
    /// The folder to output final asset
    output: PathBuf,
}

#[derive(Debug, Subcommand, Clone)]
#[command(rename_all = "lower")]
pub enum CliSubcommand {
    /// Build a fontpack definition file
    FontPack(CliFontPackCommand),
    /// Build a sprite definition file
    Sprite(CliSpriteCommand),
}

#[derive(Debug, Parser, Clone)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[clap(subcommand)]
    pub subcommand: CliSubcommand,
}

/// Parses the cli arguments
pub fn init_cli() -> anyhow::Result<CliSubcommand> {
    let args = CliArgs::try_parse().context("Failed to parse CLI arguments")?;

    Ok(args.subcommand)
}
