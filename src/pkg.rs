//! Packagers dispatcher.

/// Simple `.tar.gz` packager.
pub mod tar;

/// `.deb` packager.
pub mod deb;

/// Pacman (Arch Linux) packager – produces `.pkg.tar.zst`.
pub mod pac;

use crate::cfg::ResolvedTarget;
use crate::stage::Staging;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Trait representing packager for specific package format.
pub trait Packager
{
    /// The main function for packager. Packages the package.
    fn package(
        &self,
        staging: &Staging,
        target: &ResolvedTarget,
        out_dir: &Path,
        name: &str,
        version: &str,
    ) -> Result<PathBuf>;
}

/// Packagers factory.
pub fn packager(format: &str) -> Box<dyn Packager>
{
    match format
    {
        "tgz" => Box::new(tar::TarGzPackager),
        "deb" => Box::new(deb::DebPackager),
        "pkg" => Box::new(pac::PacmanPackager),

        _ => panic!("'{}' is not supported yet.", format),
    }
}
