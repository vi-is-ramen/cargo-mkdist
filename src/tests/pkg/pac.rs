use crate::pkg::pac::*;
use std::collections::HashMap;

#[test]
fn split_extra_field_empty()
{
    let extra = HashMap::new();
    assert_eq!(split_extra_field(&extra, "depends"), Vec::<String>::new());
}

#[test]
fn split_extra_field_works()
{
    let mut extra = HashMap::new();
    extra.insert(
        "depends".to_string(),
        toml::Value::String("glibc, openssl".to_string()),
    );
    assert_eq!(
        split_extra_field(&extra, "depends"),
        vec!["glibc".to_string(), "openssl".to_string()]
    );
}

#[test]
fn arch_from_triple_known()
{
    assert_eq!(
        arch_from_triple("x86_64-unknown-linux-gnu").unwrap(),
        "x86_64"
    );
    assert_eq!(
        arch_from_triple("aarch64-unknown-linux-gnu").unwrap(),
        "aarch64"
    );
    assert_eq!(arch_from_triple("i686-unknown-linux-gnu").unwrap(), "i686");
}

#[test]
fn arch_from_triple_unknown()
{
    let err = arch_from_triple("wasm32-unknown-unknown").unwrap_err();
    assert!(err.to_string().contains("Unsupported architecture"));
}

#[test]
fn builddate_returns_positive()
{
    assert!(builddate() > 0);
}
