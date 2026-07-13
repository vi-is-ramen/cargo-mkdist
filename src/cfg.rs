//! Configuration parsing and canonicalization.

use anyhow::Context as _;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use toml::Value;

/// Struct representing distribution target configuration.
///
/// # Fields
///
/// - `target`      ([`Option<String>`])                        - Rust triple.
///   If not specified, host triple used.
///
/// - `format`      ([`String`])                                - Target package
///   format.
///
/// - `flags`       ([`Option<String>`])                        - `cargo build`
///   flags.
///
/// - `inherit`     ([`Option<String>`])                        - Inheritance
///   base.
///
/// - `extra`       ([`HashMap<String, Value>`])                -
///   Platform/format-specific fields.
///
/// - `package`     ([`Option<String>`])                        - Package name.
///
/// - `binaries`    ([`Option<Vec<String>>`])                   - List of
///   binaries to build and put in package.
///
/// - `files`       ([`Option<Vec<(String, String, String)`])   - List of extra
///   files to copy to package.
#[derive(Debug, Deserialize, Clone)]
pub struct TargetConfig
{
    /// Rust triple. If not specified, uses host triple.
    pub target: Option<String>,

    /// Target package format.
    pub format: String,

    /// `cargo build` flags.
    pub flags: Option<String>,

    /// Inheritance base.
    pub inherit: Option<String>,

    /// Platform/format-specific fields.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,

    /// Package name.
    pub package: Option<String>,

    /// List of binaries to build and put in package.
    pub binaries: Option<Vec<String>>,

    /// List of binaries to build and put in package.
    pub files: Option<Vec<(String, String, String)>>,
}

/// Struct representing `.distinfo/targets.toml` file content.
///
/// # Fields
///
/// - `targets` ([`HashMap<String, TargetConfig>`]) - distribution targets.
#[derive(Debug, Deserialize, Clone)]
pub struct DistConfig
{
    /// Distribution targets.
    pub targets: HashMap<String, TargetConfig>,
}

impl DistConfig
{
    /// Load configuration from file.
    pub fn load(path: &Path) -> anyhow::Result<Self>
    {
        let content = std::fs::read_to_string(path)?;
        let config: DistConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Configuration search.
    pub fn find() -> anyhow::Result<PathBuf>
    {
        let candidates = [
            Path::new(".distinfo/targets.toml"),
            Path::new("dist-targets.toml"),
            Path::new(".cargo/distinfo.toml"),
        ];

        for candidate in candidates
        {
            if candidate.exists()
            {
                return Ok(candidate.to_path_buf());
            }
        }

        anyhow::bail!("`.distinfo/targets.toml` not found.")
    }
}

/// Struct representing resolved distribution target configuration.
///
/// # Fields
///
/// - `name`        ([`String`])                                - Name of
///   distribution target.
///
/// - `target`      ([`String`])                                - Rust target.
///
/// - `format`      ([`String`])                                - Target package
///   format.
///
/// - `flags`       ([`Option<String>`])                        - `cargo build`
///   flags.
///
/// - `extra`       ([`HashMap<String, Value>`])                -
///   Platform/package-specific fields.
///
/// - `package`     ([`Option<String>`])                        - Package name.
///
/// - `binaries`    ([`Option<Vec<String>>`])                   - List of
///   binaries to build and put in package.
///
/// - `files`       ([`Option<Vec<(String, String, String)`])   - List of extra
///   files to copy to package.
#[derive(Debug, Clone)]
pub struct ResolvedTarget
{
    /// Name of distribution target.
    pub name: String,

    /// Rust target.
    pub target: String,

    /// Target package format.
    pub format: String,

    /// `cargo build` flags.
    pub flags: Option<String>,

    /// Platform/package-specific fields.
    pub extra: HashMap<String, Value>,

    /// Package name.
    pub package: Option<String>,

    /// List of binaries to build and put in package.
    pub binaries: Option<Vec<String>>,

    /// List of binaries to build and put in package.
    pub files: Option<Vec<(String, String, String)>>,
}

/// Resolve distribution targets from configuration.
///
/// # Arguments
///
/// - `raw` ([`HashMap<String, TargetConfig>`]) - "raw" distribution targets
///   as-is from configuration.
///
/// # Returns
///
/// - [`anyhow::Result<HashMap<String, ResolvedTarget>>`]   - resolved
///   distribution targets.
///
/// # Errors
///
/// - If targets are inherited cyclicly, "Inheritance loop detected" error will
///   be produced.
///
/// - If some target inherits target that does not exist, "'{}' base target bot
///   found" error will be produced.
///
/// - If resolved target isn't full, "'{}' target have not '{}' and doesn't
///   inherit it." error will be produced.
pub fn resolve_targets(
    raw: HashMap<String, TargetConfig>,
) -> anyhow::Result<HashMap<String, ResolvedTarget>>
{
    let mut graph = DiGraph::<String, ()>::new();
    let mut node_indices = HashMap::new();

    // Add all targets as nodes
    for name in raw.keys()
    {
        let idx = graph.add_node(name.clone());
        node_indices.insert(name.clone(), idx);
    }

    // Inheritance edges
    for (name, config) in &raw
    {
        if let Some(parent) = &config.inherit
            && let Some(&parent_idx) = node_indices.get(parent)
            && let Some(&child_idx) = node_indices.get(name)
        {
            graph.add_edge(parent_idx, child_idx, ());
        }
    }

    // cycles check
    let sorted = toposort(&graph, None)
        .map_err(|_| anyhow::anyhow!("Inheritance loop detected."))?;

    let mut resolved: HashMap<String, ResolvedTarget> = HashMap::new();

    for node in sorted
    {
        let name = &graph[node];
        let config =
            raw.get(name).context("Can't obtain target cofniguration")?;
        let mut resolved_config = ResolvedTarget {
            name:     name.clone(),
            target:   config.target.clone().unwrap_or_default(),
            format:   config.format.clone(),
            flags:    config.flags.clone(),
            extra:    config.extra.clone(),
            package:  config.package.clone(),
            binaries: config.binaries.clone(),
            files:    config.files.clone(),
        };

        if let Some(parent) = &config.inherit
        {
            if let Some(parent_resolved) = resolved.get(parent)
            {
                if resolved_config.target.is_empty()
                {
                    resolved_config.target = parent_resolved.target.clone();
                }
                if resolved_config.flags.is_none()
                {
                    resolved_config.flags = parent_resolved.flags.clone();
                }
                // not append, but override extras
                for (k, v) in &parent_resolved.extra
                {
                    resolved_config.extra.entry(k.clone()).or_insert(v.clone());
                }
            }
            else
            {
                anyhow::bail!("'{}' base target not found.", parent);
            }
        }

        // check taht target is done
        if resolved_config.target.is_empty()
        {
            anyhow::bail!(
                "'{}' target have not 'target' and doesn't inherit it.",
                name
            );
        }

        resolved.insert(name.clone(), resolved_config);
    }

    Ok(resolved)
}
