use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};

use crate::output::OutputType;

#[derive(Debug, Args, Clone)]
pub struct CliFontPackCommand {
    /// The fontpack defintion file
    #[clap(short, long)]
    pub definition: PathBuf,
    /// The folder to output final asset
    #[clap(short, long)]
    pub output: PathBuf,
    #[clap(short = 't', long)]
    pub output_type: OutputType,
}

#[derive(Debug, Args, Clone)]
pub struct CliSpriteCommand {
    /// The sprite table file
    #[clap(short, long)]
    pub definition: PathBuf,
    /// The folder to output final asset
    #[clap(short, long)]
    pub output: PathBuf,
    #[clap(short = 't', long)]
    pub output_type: OutputType,
    #[clap(short, long)]
    pub palette: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct CliPaletteCommand {
    /// Where the palette file is written to
    #[clap(short, long)]
    pub output: PathBuf,
    /// The folder where the c header and file is output
    #[clap(short = 'c', long)]
    pub output_c_folder: PathBuf,
    /// The sprite definition to generate the palette from
    #[clap(short, long, num_args = 1.., required = true)]
    pub definitions: Vec<PathBuf>,
}

#[derive(Debug, Subcommand, Clone)]
#[command(rename_all = "lower")]
pub enum CliSubcommand {
    /// Build a fontpack definition
    FontPack(CliFontPackCommand),
    /// Build a sprite definition
    Sprite(CliSpriteCommand),
    /// Generate a palette definition file
    Palette(CliPaletteCommand),
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
