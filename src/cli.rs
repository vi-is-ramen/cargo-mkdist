//! Command-line interface for mkdist.

use clap::Parser;

/// Structure representing CLI arguments.
///
/// # Fields
///
/// - `#[arg(required = false)] target` (`Option<String>`) - Target platform to
///   dist to.
///
/// - `default_value_t = false)] all` (`bool`) - Collect all available targets.
///
/// - `#[arg(long)] list` (`bool`) - List available targets.
///
/// - `#[arg(long)] debug` (`bool`) - Build in debug mode.
///
/// - `default_value = "target/packages")] out_dir` (`String`) - Specify custom
///   output directory (`target/packages`` by default).
///
/// - `#[arg(last = true)] cargo_args` (`Vec<String>`) - `cargo build` flags.
#[derive(Parser, Debug, Clone)]
#[command(name = "cargo mkdist")]
#[command(about = "Build distribution packages for Rust projects", version)]
pub struct Cli
{
    /// Target platform to dist to.
    #[arg(required = false)]
    pub target: Option<String>,

    /// Collect all available targets.
    #[arg(short, long, default_value_t = false)]
    pub all: bool,

    /// List available targets.
    #[arg(long)]
    pub list: bool,

    /// Build in debug mode.
    #[arg(long)]
    pub debug: bool,

    /// Specify custom output directory (`target/packages`` by default).
    #[arg(short, long, default_value = "target/packages")]
    pub out_dir: String,

    /// `cargo build` flags.
    #[arg(last = true)]
    pub cargo_args: Vec<String>,
}
