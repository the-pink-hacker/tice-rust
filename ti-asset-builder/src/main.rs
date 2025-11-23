#![feature(normalize_lexically)]

mod cli;
mod font;
mod output;
mod path;
mod sprite;

fn main() -> anyhow::Result<()> {
    let subcommand = cli::init_cli()?;

    match subcommand {
        cli::CliSubcommand::FontPack(command) => font::build(command),
        cli::CliSubcommand::Sprite(command) => sprite::build(command),
    }
}
