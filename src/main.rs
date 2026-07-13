#![warn(missing_docs)]
#![doc = include_str!("../DOCS.md")]

mod meta;

mod build;

mod stage;

mod pkg;

mod cfg;

mod cli;

#[cfg(test)]
mod tests;

use clap::Parser as _;

use anyhow::{Context as _, Result};

fn main() -> Result<()>
{
    let cli = cli::Cli::parse();

    if cli.list
    {
        let config_path = cfg::DistConfig::find()?;
        let config = cfg::DistConfig::load(&config_path)?;
        println!("Available targets:");
        for name in config.targets.keys()
        {
            println!("  {}", name);
        }
        return Ok(());
    }

    let config_path = cfg::DistConfig::find()?;
    let raw_config = cfg::DistConfig::load(&config_path)?;

    let resolved_targets = cfg::resolve_targets(raw_config.targets)?;

    let targets_to_build: Vec<String> = if cli.all
    {
        resolved_targets.keys().cloned().collect()
    }
    else if let Some(target_name) = &cli.target
    {
        if !resolved_targets.contains_key(target_name)
        {
            anyhow::bail!("'{}' target not found.", target_name);
        }
        vec![target_name.clone()]
    }
    else
    {
        if let Some(default) = resolved_targets.get("default")
        {
            vec![default.name.clone()]
        }
        else
        {
            anyhow::bail!(
                "No target is specified, and there is no 'default' target in \
                 configuration. Use --list to view the targets or --all to \
                 build all."
            )
        }
    };

    let metadata = meta::get()?;

    for name in targets_to_build
    {
        let target = resolved_targets
            .get(&name)
            .context("Can't get resolved target.")?;
        println!(
            "   Compiling '{}' ({}-{})",
            name, target.format, target.target
        );

        let mut cargo_flags: Vec<String> = Vec::new();
        if let Some(flags_str) = &target.flags
        {
            cargo_flags.extend(flags_str.split_whitespace().map(String::from));
        }
        cargo_flags.extend(cli.cargo_args.clone());

        build::build_project(
            &target.target,
            &cargo_flags,
            cli.debug,
            target.package.as_deref(),
        )?;

        let staging = stage::Staging::new()?;

        let binary_names = if let Some(ref bins) = target.binaries
        {
            bins.clone()
        }
        else
        {
            let pkg_name = target.package.as_deref().map(|s| s.to_string());
            meta::get_binary_names(pkg_name)?
        };

        for name in &binary_names
        {
            staging.copy_binary(name, &target.target, cli.debug)?;
        }

        let files = if let Some(ref files) = target.files
        {
            files.clone()
        }
        else
        {
            vec![]
        };

        staging.copy_extra_files(&files)?;

        let packager = pkg::packager(&target.format);
        let out_dir = std::path::Path::new(&cli.out_dir).join(&name);
        let package_meta =
            meta::find_package(&metadata, target.package.clone())?;
        let output_path = packager.package(
            &staging,
            target,
            &out_dir,
            &package_meta.name,
            &package_meta.version.to_string(),
        )?;
        println!("    Finished {}.", output_path.display());
    }

    Ok(())
}
