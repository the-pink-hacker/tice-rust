#![feature(normalize_lexically)]
#![feature(iterator_try_collect)]
#![feature(vec_into_chunks)]
#![feature(iter_intersperse)]

mod cli;
mod font;
mod image;
mod output;
mod palette;
mod path;
mod sprite;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let subcommand = cli::init_cli()?;

    match subcommand {
        cli::CliSubcommand::FontPack(command) => font::build(command).await,
        cli::CliSubcommand::Sprite(command) => sprite::build(command).await,
        cli::CliSubcommand::Palette(command) => palette::build(command).await,
    }
}
