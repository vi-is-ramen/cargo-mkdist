//! Build function for distribution pipeline.

/// Build specified project.
///
/// # Arguments
///
/// - `target` (`&str`) - Rust target.
/// - `flags` (`&[String]`) - Cargo flags.
/// - `debug` (`bool`) - Debug mode flag.
/// - `package` (`Option<&str>`) - Package name.
///
/// # Returns
///
/// - `anyhow::Result<()>` - [`Ok`] on success or [`anyhow::Error`] on failure.
///
/// # Errors
///
/// - If Cargo failed, it's error will be produced.
pub fn build_project(
    target: &str,
    flags: &[String],
    debug: bool,
    package: Option<&str>,
) -> anyhow::Result<()>
{
    let mut cmd = std::process::Command::new("cargo");

    cmd.arg("build");

    if !debug
    {
        cmd.arg("--release");
    }

    cmd.arg("--target").arg(target);

    if let Some(pkg) = package
    {
        cmd.arg("--package").arg(pkg);
    }

    if !flags.is_empty()
    {
        cmd.args(flags);
    }

    let status = cmd.status()?;

    if !status.success()
    {
        anyhow::bail!("Build failed.");
    }

    Ok(())
}
