//! Cargo project metadata inspecting.

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, Package};

/// Obtain package/workspace metadata.
///
/// # Returns
///
/// - `Result<Metadata>` - Package/workspace metadata.
///
/// # Errors
///
/// - If `cargo metadata` failed in some reason, it's error will be produced.
pub fn get() -> Result<Metadata>
{
    let cmd = cargo_metadata::MetadataCommand::new();

    cmd.exec().context("Make sure you are in package root now.")
}

/// Find pacakge by its name.
///
/// # Arguments
///
/// - `metadata` (`&'a Metadata`) - Workspace metadata.
/// - `package_name` (`Option<String>`) - Package name.
///
/// # Returns
///
/// - `Result<&'a Package>` - Package metadata.
///
/// # Errors
///
/// - Package missing in workspace => "'{}' not found in workspace"
///
/// - Active package not found => "Can't detect root pacakge. Are you in
///   workspace subdirectory?"
///
/// - Package manifest missing or corrupted => "Package root not found."
pub fn find_package(
    metadata: &Metadata,
    package_name: Option<String>,
) -> Result<&Package>
{
    match package_name
    {
        Some(name) => metadata
            .packages
            .iter()
            .find(|pkg| pkg.name == name)
            .with_context(|| format!("'{}' not found in workspace", name)),

        None =>
        {
            let root_id = metadata
                .resolve
                .as_ref()
                .and_then(|res| res.root.as_ref())
                .context(
                    "Can't detect root pacakge. Are you in workspace \
                     subdirectory?",
                )?;

            metadata
                .packages
                .iter()
                .find(|pkg| pkg.id == *root_id)
                .context("Package root not found.")
        },
    }
}

/// Obtain list of binaries package exposes.
///
/// # Arguments
///
/// - `package` (`Option<String>`) - Package name.
///
/// # Returns
///
/// - `Result<Vec<String>>` - List of binary names.
///
/// # Errors
///
/// - Metadata obtaining failed => propagation.
///
/// - Package obtaining failed => propagation.
pub fn get_binary_names(package: Option<String>) -> Result<Vec<String>>
{
    let metadata = get()?;
    let pkg = find_package(&metadata, package)?;

    let bins: Vec<String> = pkg
        .targets
        .iter()
        .filter(|target| target.kind.contains(&cargo_metadata::TargetKind::Bin))
        .map(|target| target.name.clone())
        .collect();

    if bins.is_empty()
    {
        eprintln!("'{}' package have not binary targets.", pkg.name);
    }

    Ok(bins)
}
