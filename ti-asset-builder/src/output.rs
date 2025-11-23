#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputType {
    /// A fasmg compatible assembly file.
    Assembly,
    /// The raw binary asset with no header.
    Binary,
    /// A C header file.
    C,
}
