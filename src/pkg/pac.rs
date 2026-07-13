//! Pacman (.pkg.tar.zst) packager for Arch Linux.

use super::Packager;
use crate::cfg::ResolvedTarget;
use crate::meta::{find_package, get as get_metadata};
use crate::stage::Staging;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Builder;
use walkdir::WalkDir;
use zstd::stream::write::Encoder as ZstdEncoder;

/// Arch Linux pacman packager.
pub struct PacmanPackager;

impl Packager for PacmanPackager
{
    fn package(
        &self,
        staging: &Staging,
        target: &ResolvedTarget,
        out_dir: &Path,
        name: &str,
        version: &str,
    ) -> Result<PathBuf>
    {
        // We don't support Linux distribution on non-UNIX so far.
        #[cfg(not(unix))]
        panic!("Linux distributions are not supported on non-UNIX so far.");

        // Get package metadata from Cargo
        let metadata = get_metadata()?;
        let pkg_meta = find_package(&metadata, target.package.clone())?;

        // Architecture from target triple
        let arch = arch_from_triple(&target.target)?;

        // Extract extra fields
        let pkgrel = target
            .extra
            .get("pkgrel")
            .and_then(|v| v.as_integer())
            .unwrap_or(1)
            .to_string();
        let epoch = target
            .extra
            .get("epoch")
            .and_then(|v| v.as_integer())
            .map(|e| e.to_string());
        let pkgdesc = pkg_meta
            .description
            .clone()
            .unwrap_or_else(|| "Rust package".to_string());
        let license = target
            .extra
            .get("license")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let url = target
            .extra
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let packager = pkg_meta.authors.join(", ");

        // Dependencies, etc. (comma separated)
        let depends = split_extra_field(&target.extra, "depends");
        let optdepends = split_extra_field(&target.extra, "optdepends");
        let conflicts = split_extra_field(&target.extra, "conflicts");
        let provides = split_extra_field(&target.extra, "provides");
        let replaces = split_extra_field(&target.extra, "replaces");
        let backup = split_extra_field(&target.extra, "backup");

        // Install script
        let install_script = target
            .extra
            .get("install")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        // Prepare staging root
        let staging_root = staging.root.path().to_path_buf();

        // 1. Compute total size of files (installed size)
        let mut total_size = 0u64;
        let mut file_entries: Vec<(PathBuf, fs::Metadata)> = Vec::new();
        for entry in WalkDir::new(&staging_root)
            .into_iter()
            .filter_entry(|e| e.path() != staging_root)
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
            {
                let meta = fs::metadata(path)?;
                total_size += meta.len();
                file_entries.push((path.to_path_buf(), meta));
            }
        }

        // 2. Create .PKGINFO
        let pkg_info_path = staging_root.join(".PKGINFO");
        {
            let mut f = File::create(&pkg_info_path)?;
            writeln!(f, "pkgname = {}", name)?;
            if let Some(epoch) = &epoch
            {
                writeln!(f, "epoch = {}", epoch)?;
            }
            writeln!(f, "pkgver = {}-{}", version, pkgrel)?;
            writeln!(f, "pkgdesc = {}", pkgdesc)?;
            writeln!(f, "arch = {}", arch)?;
            writeln!(f, "license = {}", license)?;
            if !url.is_empty()
            {
                writeln!(f, "url = {}", url)?;
            }
            writeln!(f, "builddate = {}", builddate())?;
            writeln!(f, "packager = {}", packager)?;
            writeln!(f, "size = {}", total_size)?; // will be updated with compressed size later
            // Dependencies
            for dep in &depends
            {
                writeln!(f, "depend = {}", dep)?;
            }
            for opt in &optdepends
            {
                writeln!(f, "optdepend = {}", opt)?;
            }
            for c in &conflicts
            {
                writeln!(f, "conflict = {}", c)?;
            }
            for p in &provides
            {
                writeln!(f, "provides = {}", p)?;
            }
            for r in &replaces
            {
                writeln!(f, "replaces = {}", r)?;
            }
            for b in &backup
            {
                writeln!(f, "backup = {}", b)?;
            }
        }

        // 3. Copy install script if present
        if let Some(script_path) = install_script
        {
            if script_path.exists()
            {
                let dest = staging_root.join(".INSTALL");
                fs::copy(&script_path, &dest)?;
                // Make executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&dest)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&dest, perms)?;
                }
            }
            else
            {
                eprintln!(
                    "warning: .INSTALL script '{}' not found, skipping.",
                    script_path.display()
                );
            }
        }

        // 4. Create archive
        let output_file = out_dir.join(format!(
            "{}-{}-{}-{}.pkg.tar.zst",
            name, version, pkgrel, arch
        ));
        fs::create_dir_all(out_dir)?;

        // Open file, create zstd encoder
        let file = File::create(&output_file)?;
        let encoder = ZstdEncoder::new(file, 19)?;
        let mut tar_builder = Builder::new(encoder);

        // Add all files from staging
        let root = staging_root.clone();
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| e.path() != root)
        {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path.strip_prefix(&root)?;
            let meta = entry.metadata()?;

            if meta.is_file()
            {
                let mut f = File::open(path)?;
                tar_builder.append_file(rel_path, &mut f)?;
            }
            else if meta.is_dir()
            {
                // Add directory with permissions
                tar_builder.append_dir(rel_path, path)?;
            }
            else if meta.is_symlink()
            {
                // Read link target and add as symlink
                let target = fs::read_link(path)?;
                let mut header = tar::Header::new_gnu();
                tar_builder.append_link(&mut header, rel_path, target)?;
            }
        }

        // Finish tar and zstd
        tar_builder.into_inner()?;
        // zstd encoder finishes on drop

        Ok(output_file)
    }
}

/// Split a comma-separated extra field into a vector of trimmed strings.
pub fn split_extra_field(
    extra: &HashMap<String, toml::Value>,
    key: &str,
) -> Vec<String>
{
    extra
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default()
}

/// Get current timestamp in seconds since epoch (for builddate).
pub fn builddate() -> u64
{
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Determine pacman architecture from Rust target triple.
pub fn arch_from_triple(triple: &str) -> Result<String>
{
    let arch = match triple.split('-').next().unwrap_or("")
    {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "armv7" => "armv7h",
        "arm" => "armv7h", // fallback
        "i686" | "i586" | "i386" => "i686",
        "riscv64" => "riscv64",
        other =>
        {
            anyhow::bail!("Unsupported architecture for pacman: {}", other)
        },
    };
    Ok(arch.to_string())
}
