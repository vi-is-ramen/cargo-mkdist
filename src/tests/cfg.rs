use crate::cfg::*;

use std::collections::HashMap;

fn make_target_config(
    target: Option<&str>,
    format: &str,
    inherit: Option<&str>,
    extra: Vec<(&str, &str)>,
) -> TargetConfig
{
    let mut extra_map = HashMap::new();
    for (k, v) in extra
    {
        extra_map.insert(k.to_string(), toml::Value::String(v.to_string()));
    }
    TargetConfig {
        target:   target.map(String::from),
        format:   format.to_string(),
        flags:    None,
        inherit:  inherit.map(String::from),
        extra:    extra_map,
        package:  None,
        binaries: None,
        files:    None,
    }
}

#[test]
fn resolve_no_inheritance()
{
    let mut raw = HashMap::new();
    raw.insert(
        "test".to_string(),
        make_target_config(
            Some("x86_64-unknown-linux-gnu"),
            "deb",
            None,
            vec![],
        ),
    );
    let resolved = resolve_targets(raw).unwrap();
    assert_eq!(resolved.len(), 1);
    let t = resolved.get("test").unwrap();
    assert_eq!(t.target, "x86_64-unknown-linux-gnu");
    assert_eq!(t.format, "deb");
}

#[test]
fn resolve_inheritance_simple()
{
    let mut raw = HashMap::new();
    raw.insert(
        "base".to_string(),
        make_target_config(
            Some("x86_64-unknown-linux-gnu"),
            "tgz",
            None,
            vec![],
        ),
    );
    raw.insert(
        "child".to_string(),
        make_target_config(
            None,
            "deb",
            Some("base"),
            vec![("section", "utils")],
        ),
    );
    let resolved = resolve_targets(raw).unwrap();
    let child = resolved.get("child").unwrap();
    assert_eq!(child.target, "x86_64-unknown-linux-gnu");
    assert_eq!(child.format, "deb");
    assert_eq!(
        child.extra.get("section").and_then(|v| v.as_str()),
        Some("utils")
    );
}

#[test]
fn resolve_inheritance_override()
{
    let mut raw = HashMap::new();
    raw.insert(
        "base".to_string(),
        make_target_config(Some("i686-unknown-linux-gnu"), "pkg", None, vec![]),
    );
    raw.insert(
        "child".to_string(),
        make_target_config(
            Some("x86_64-unknown-linux-gnu"),
            "deb",
            Some("base"),
            vec![],
        ),
    );
    let resolved = resolve_targets(raw).unwrap();
    let child = resolved.get("child").unwrap();
    assert_eq!(child.target, "x86_64-unknown-linux-gnu");
    assert_eq!(child.format, "deb");
}

#[test]
fn resolve_cyclic_error()
{
    let mut raw = HashMap::new();
    raw.insert(
        "a".to_string(),
        make_target_config(None, "deb", Some("b"), vec![]),
    );
    raw.insert(
        "b".to_string(),
        make_target_config(None, "deb", Some("a"), vec![]),
    );
    let err = resolve_targets(raw).unwrap_err();
    assert!(err.to_string().contains("Inheritance loop"));
}

#[test]
fn resolve_missing_parent_error()
{
    let mut raw = HashMap::new();
    raw.insert(
        "child".to_string(),
        make_target_config(None, "deb", Some("missing"), vec![]),
    );
    let err = resolve_targets(raw).unwrap_err();
    assert!(err.to_string().contains("'missing' base target not found"));
}

#[test]
fn resolve_missing_target_error()
{
    let mut raw = HashMap::new();
    raw.insert(
        "test".to_string(),
        make_target_config(None, "deb", None, vec![]),
    );
    let err = resolve_targets(raw).unwrap_err();
    assert!(err.to_string().contains("have not 'target'"));
}
