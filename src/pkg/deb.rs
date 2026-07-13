//! Debian (.deb) packager.

use super::Packager;
use crate::cfg::ResolvedTarget;
use crate::meta::{find_package, get as get_metadata};
use crate::stage::Staging;
use anyhow::Result;
use ar::Builder as ArBuilder;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::Builder;
use walkdir::WalkDir;

pub struct ControlArgs<'a>
{
    f:              Box<File>,
    package:        &'a str,
    version:        &'a str,
    arch:           &'a str,
    section:        &'a str,
    priority:       &'a str,
    maintainer:     &'a str,
    description:    &'a str,
    installed_size: u64,
    depends:        &'a [String],
    recommends:     &'a [String],
    suggests:       &'a [String],
    conflicts:      &'a [String],
    breaks:         &'a [String],
    replaces:       &'a [String],
    provides:       &'a [String],
    enhances:       &'a [String],
    pre_depends:    &'a [String],
}

/// Debian packager – produces `.deb` files.
pub struct DebPackager;

impl Packager for DebPackager
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

        // Get metadata from Cargo
        let metadata = get_metadata()?;
        let pkg_meta = find_package(&metadata, target.package.clone())?;

        // Architecture from target triple
        let arch = deb_arch_from_triple(&target.target)?;

        // Extract control fields from extra
        let extra = &target.extra;
        let section = extra
            .get("section")
            .and_then(|v| v.as_str())
            .unwrap_or("utils")
            .to_string();
        let priority = extra
            .get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("optional")
            .to_string();
        let maintainer = extra
            .get("maintainer")
            .map(|v| v.to_string())
            .unwrap_or_else(|| pkg_meta.authors.join(", "))
            .to_string();
        let description = extra
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                pkg_meta.description.as_deref().unwrap_or("Rust package")
            })
            .to_string();

        // Dependencies (comma-separated)
        let depends = split_deb_field(extra, "depends");
        let recommends = split_deb_field(extra, "recommends");
        let suggests = split_deb_field(extra, "suggests");
        let conflicts = split_deb_field(extra, "conflicts");
        let breaks = split_deb_field(extra, "breaks");
        let replaces = split_deb_field(extra, "replaces");
        let provides = split_deb_field(extra, "provides");
        let enhances = split_deb_field(extra, "enhances");
        let pre_depends = split_deb_field(extra, "pre-depends");

        // Conffiles – list of files that should be treated as configuration
        // files
        let conffiles = split_deb_field(extra, "conffiles");

        // Maintainer scripts
        let scripts = get_scripts(extra);

        // Compute installed size (in KiB, rounded up)
        let installed_size = compute_installed_size(staging)?;

        // Prepare staging directories
        let staging_root = staging.root.path();

        // 1. Build control.tar.gz
        let control_tar_gz = build_control_tarball(
            ControlArgs {
                f: Box::new(tempfile::tempfile()?),
                package: name,
                version,
                arch: &arch,
                section: &section,
                priority: &priority,
                maintainer: &maintainer,
                description: &description,
                installed_size,
                depends: &depends,
                recommends: &recommends,
                suggests: &suggests,
                conflicts: &conflicts,
                breaks: &breaks,
                replaces: &replaces,
                provides: &provides,
                enhances: &enhances,
                pre_depends: &pre_depends,
            },
            &conffiles,
            &scripts,
        )?;

        let mut metadata = File::open(control_tar_gz)?;

        // 2. Build data.tar.gz from staging
        let data_tar_gz = build_data_tarball(staging_root)?;

        // 3. Create .deb (ar archive)
        let output_path =
            out_dir.join(format!("{}_{}_{}.deb", name, version, arch));
        fs::create_dir_all(out_dir)?;

        let mut ar_builder = ArBuilder::new(File::create(&output_path)?);

        let mut data = File::open(data_tar_gz)?;

        let mut binary = tempfile::tempfile()?;

        binary.write_all(&b"2.0\n"[..])?;

        // add debian-binary
        ar_builder.append_file(b"debian-binary", &mut binary)?;

        // Add control.tar.gz
        ar_builder.append_file(b"control.tar.gz", &mut metadata)?;
        // Add data.tar.gz
        ar_builder.append_file(b"data.tar.gz", &mut data)?;

        Ok(output_path)
    }
}

/// Build control.tar.gz with metadata and scripts.
pub fn build_control_tarball(
    mut args: ControlArgs,
    conffiles: &[String],
    scripts: &HashMap<String, PathBuf>,
) -> Result<PathBuf>
{
    // Create temporary file for control.tar.gz
    let temp_dir = tempfile::tempdir()?;
    let control_path = temp_dir.path().join("control");

    args.f = Box::new(File::create(&control_path)?);

    // Write control file
    write_control(args)?;

    // Write conffiles if any
    if !conffiles.is_empty()
    {
        let conffile_path = temp_dir.path().join("conffiles");
        let mut conffile = File::create(&conffile_path)?;
        for conf in conffiles
        {
            writeln!(conffile, "{}", conf)?;
        }
        // conffiles will be added to tarball
    }

    // Copy scripts to temp dir
    for (name, path) in scripts
    {
        let dest = temp_dir.path().join(name);
        fs::copy(path, &dest)?;
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest, perms)?;
        }
    }

    // Pack all files from temp_dir into control.tar.gz
    let output = temp_dir.path().join("control.tar.gz");
    let tar_gz = File::create(&output)?;
    let gz = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(gz);

    for entry in WalkDir::new(temp_dir.path())
        .into_iter()
        .filter_entry(|e| e.path() != temp_dir.path())
    {
        let entry = entry?;
        let path = entry.path();
        let rel_path = path.strip_prefix(temp_dir.path())?;
        if path.is_file()
        {
            tar.append_file(rel_path, &mut File::open(path)?)?;
        }
    }

    tar.into_inner()?; // finish gz
    Ok(output)
}

/// Write the `control` file.
pub fn write_control(mut args: ControlArgs) -> Result<()>
{
    writeln!(args.f, "Package: {}", args.package)?;
    writeln!(args.f, "Version: {}", args.version)?;
    writeln!(args.f, "Architecture: {}", args.arch)?;
    writeln!(args.f, "Maintainer: {}", args.maintainer)?;
    writeln!(args.f, "Installed-Size: {}", args.installed_size)?;
    writeln!(args.f, "Section: {}", args.section)?;
    writeln!(args.f, "Priority: {}", args.priority)?;

    // Optional fields
    if !args.depends.is_empty()
    {
        writeln!(args.f, "Depends: {}", args.depends.join(", "))?;
    }
    if !args.pre_depends.is_empty()
    {
        writeln!(args.f, "Pre-Depends: {}", args.pre_depends.join(", "))?;
    }
    if !args.recommends.is_empty()
    {
        writeln!(args.f, "Recommends: {}", args.recommends.join(", "))?;
    }
    if !args.suggests.is_empty()
    {
        writeln!(args.f, "Suggests: {}", args.suggests.join(", "))?;
    }
    if !args.conflicts.is_empty()
    {
        writeln!(args.f, "Conflicts: {}", args.conflicts.join(", "))?;
    }
    if !args.breaks.is_empty()
    {
        writeln!(args.f, "Breaks: {}", args.breaks.join(", "))?;
    }
    if !args.replaces.is_empty()
    {
        writeln!(args.f, "Replaces: {}", args.replaces.join(", "))?;
    }
    if !args.provides.is_empty()
    {
        writeln!(args.f, "Provides: {}", args.provides.join(", "))?;
    }
    if !args.enhances.is_empty()
    {
        writeln!(args.f, "Enhances: {}", args.enhances.join(", "))?;
    }

    // Description: first line is summary, then extended description indented
    // with space
    let desc_lines: Vec<&str> = args.description.lines().collect();
    if let Some(first) = desc_lines.first()
    {
        writeln!(args.f, "Description: {}", first)?;
        for line in desc_lines.iter().skip(1)
        {
            writeln!(args.f, " {}", line)?;
        }
    }

    Ok(())
}

/// Build data.tar.gz from staging directory.
pub fn build_data_tarball(staging_root: &Path) -> Result<PathBuf>
{
    let temp_dir = tempfile::tempdir()?;
    let output = temp_dir.path().join("data.tar.gz");
    let tar_gz = File::create(&output)?;
    let gz = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(gz);

    // Walk staging root, skipping root itself
    for entry in WalkDir::new(staging_root)
        .into_iter()
        .filter_entry(|e| e.path() != staging_root)
    {
        let entry = entry?;
        let path = entry.path();
        let rel_path = path.strip_prefix(staging_root)?;
        let meta = entry.metadata()?;

        if meta.is_file()
        {
            tar.append_file(rel_path, &mut File::open(path)?)?;
        }
        else if meta.is_dir()
        {
            tar.append_dir(rel_path, path)?;
        }
        else if meta.is_symlink()
        {
            let target = fs::read_link(path)?;
            let mut h = tar::Header::new_gnu();
            tar.append_link(&mut h, rel_path, target)?;
        }
    }

    tar.into_inner()?;
    Ok(output)
}

/// Compute installed size in KiB (rounded up).
pub fn compute_installed_size(staging: &Staging) -> Result<u64>
{
    let mut total_bytes = 0u64;
    for entry in WalkDir::new(staging.root.path()).into_iter()
    {
        let entry = entry?;
        if entry.path().is_file()
        {
            total_bytes += entry.metadata()?.len();
        }
    }
    // Round up to KiB (1 KiB = 1024 bytes)
    let kib = total_bytes.div_ceil(1024);
    Ok(kib)
}

/// Split a comma-separated field from extra into a vector of trimmed strings.
pub fn split_deb_field(
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

/// Get maintainer scripts from extra.
pub fn get_scripts(
    extra: &HashMap<String, toml::Value>,
) -> HashMap<String, PathBuf>
{
    let mut map = HashMap::new();
    // Script names: preinst, postinst, prerm, postrm
    for script in &["preinst", "postinst", "prerm", "postrm"]
    {
        if let Some(path) = extra
            .get(*script)
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
        {
            if path.exists()
            {
                map.insert(script.to_string(), path);
            }
            else
            {
                eprintln!("warning: script '{}' not found, skipping.", script);
            }
        }
    }
    map
}

/// Determine Debian architecture from Rust target triple.
pub fn deb_arch_from_triple(triple: &str) -> Result<String>
{
    let arch = match triple.split('-').next().unwrap_or("")
    {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "armv7" => "armhf",
        "arm" => "armel",
        "i686" | "i586" | "i386" => "i386",
        "powerpc64le" => "ppc64el",
        "s390x" => "s390x",
        "mips64el" => "mips64el",
        "riscv64" => "riscv64",
        other =>
        {
            anyhow::bail!("Unsupported architecture for Debian: {}", other)
        },
    };
    Ok(arch.to_string())
}
