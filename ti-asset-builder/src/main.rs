#![feature(normalize_lexically)]

mod cli;
mod font;
mod output;
mod path;
mod sprite;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let subcommand = cli::init_cli()?;

    match subcommand {
        cli::CliSubcommand::FontPack(command) => font::build(command).await,
        cli::CliSubcommand::Sprite(command) => sprite::build(command).await,
    }
}
