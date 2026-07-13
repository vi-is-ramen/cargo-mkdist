//! TGZ packager

use super::Packager;
use crate::cfg::ResolvedTarget;
use crate::stage::Staging;
use anyhow::Result;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Builder;

/// Simple `.tar.gz` packager.
pub struct TarGzPackager;

impl Packager for TarGzPackager
{
    fn package(
        &self,
        staging: &Staging,
        _target: &ResolvedTarget,
        out_dir: &Path,
        name: &str,
        version: &str,
    ) -> Result<PathBuf>
    {
        std::fs::create_dir_all(out_dir)?;
        let output = out_dir.join(format!("{}-{}.tar.gz", name, version));

        let tar_gz = File::create(&output)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(enc);

        tar.append_dir_all(".", staging.root.path())?;

        let _ = tar.into_inner()?;

        Ok(output)
    }
}
