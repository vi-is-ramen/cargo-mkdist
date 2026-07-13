//! Staging.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

/// Struct represents staging.
///
/// # Fields
///
/// - `root` (`tempfile`) - Path to package root directory.
pub struct Staging
{
    /// Path to package root directory.
    pub root: tempfile::TempDir,
}

impl Staging
{
    /// Construct new [`Staging`] struct.
    ///
    /// # Returns
    ///
    /// - `anyhow::Result<Self>` - [`Staging`] struct or [`anyhow`]'s error.
    ///
    /// # Errors
    ///
    /// - If temporary directory creation failed, it's error will be produced.
    pub fn new() -> anyhow::Result<Self>
    {
        let dir = tempfile::tempdir()?;
        Ok(Self { root: dir })
    }

    /// Copy binary to package's `/usr/bin/` and make it executable (if
    /// applicable).
    ///
    /// # Arguments
    ///
    /// - `binary_name`     ([`&str`]) -    Name of binary.
    /// - `target_triple`   ([`&str`]) -    Rust target.
    /// - `debug`           ([`bool`]) -    Set if binary should be found in
    ///   debug directory.
    ///
    /// # Returns
    ///
    /// - `anyhow::Result<PathBuf>` - Real path to the copied binary.
    ///
    /// # Errors
    ///
    /// - If binary doesn't exist, "Binary not found: {}" error will be
    ///   produced.
    ///
    /// - If copying failed, it's error will be produced.
    ///
    /// - If mode change failed, it's error will be produced.
    ///
    /// # Platform-specific behavior
    ///
    /// This function sets mode `0o755` for UNIX-like platforms using stardard
    /// [`std::os::unix`] interface. It might fail, but should not actually.
    /// On all other platforms this behavior is disabled.
    pub fn copy_binary(
        &self,
        binary_name: &str,
        target_triple: &str,
        debug: bool,
    ) -> anyhow::Result<PathBuf>
    {
        let build_dir = if debug { "debug" } else { "release" };

        let src = Path::new("target")
            .join(target_triple)
            .join(build_dir)
            .join(binary_name);

        if !src.exists()
        {
            anyhow::bail!("Binary not found: {:?}", src);
        }

        let dest = self.root.path().join("usr/bin").join(binary_name);

        std::fs::create_dir_all(dest.parent().context("Invalid path.")?)?;
        std::fs::copy(&src, &dest)?;

        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms)?;
        }

        Ok(dest)
    }

    /// Copy extra files into package.
    ///
    /// # Arguments
    ///
    /// - `files` (`&HashMap<String, String>`) - Map of files (destination is
    ///   key, source is value).
    ///
    /// # Returns
    ///
    /// - `anyhow::Result<()>` - [`Ok`] if everything is done, [`anyhow::Error`]
    ///   otherwise.
    ///
    /// # Errors
    ///
    /// - if target directory creation failed or copying failed, operation's
    ///   error will be produced.
    pub fn copy_extra_files(
        &self,
        files: &Vec<(String, String, String)>,
    ) -> anyhow::Result<()>
    {
        for (src, dest, perm) in files
        {
            let src_path = Path::new(src);
            let dest_path = self.root.path().join(dest);
            std::fs::create_dir_all(
                dest_path.parent().context("Invalid path.")?,
            )?;
            std::fs::copy(src_path, &dest_path)?;

            #[cfg(unix)]
            {
                let mut perms = std::fs::metadata(&dest_path)?.permissions();
                perms.set_mode(u32::from_str_radix(perm, 8)?);
                std::fs::set_permissions(&dest_path, perms)?;
            }
        }
        Ok(())
    }
}
