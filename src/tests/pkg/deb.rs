use crate::pkg::deb::*;
use crate::stage::Staging;
use std::collections::HashMap;

#[test]
fn split_deb_field_empty()
{
    let extra = HashMap::new();
    assert_eq!(split_deb_field(&extra, "depends"), Vec::<String>::new());
}

#[test]
fn split_deb_field_single()
{
    let mut extra = HashMap::new();
    extra.insert(
        "depends".to_string(),
        toml::Value::String("libc6".to_string()),
    );
    assert_eq!(
        split_deb_field(&extra, "depends"),
        vec!["libc6".to_string()]
    );
}

#[test]
fn split_deb_field_multiple()
{
    let mut extra = HashMap::new();
    extra.insert(
        "depends".to_string(),
        toml::Value::String("libc6 (>= 2.34), libssl3,  zlib ".to_string()),
    );
    assert_eq!(
        split_deb_field(&extra, "depends"),
        vec![
            "libc6 (>= 2.34)".to_string(),
            "libssl3".to_string(),
            "zlib".to_string()
        ]
    );
}

#[test]
fn deb_arch_from_triple_known()
{
    assert_eq!(
        deb_arch_from_triple("x86_64-unknown-linux-gnu").unwrap(),
        "amd64"
    );
    assert_eq!(
        deb_arch_from_triple("aarch64-unknown-linux-gnu").unwrap(),
        "arm64"
    );
    assert_eq!(
        deb_arch_from_triple("armv7-unknown-linux-gnueabihf").unwrap(),
        "armhf"
    );
    assert_eq!(
        deb_arch_from_triple("i686-unknown-linux-gnu").unwrap(),
        "i386"
    );
}

#[test]
fn deb_arch_from_triple_unknown()
{
    let err = deb_arch_from_triple("wasm32-unknown-unknown").unwrap_err();
    assert!(err.to_string().contains("Unsupported architecture"));
}

#[test]
fn compute_installed_size_works() -> anyhow::Result<()>
{
    let staging = Staging::new()?;
    let subdir = staging.root.path().join("usr/bin");
    std::fs::create_dir_all(&subdir)?;
    let file1 = subdir.join("a");
    let file2 = subdir.join("b");
    std::fs::write(&file1, vec![0u8; 1024])?; // 1 KiB
    std::fs::write(&file2, vec![0u8; 512])?; // 0.5 KiB -> should round up to 2 KiB
    let size = compute_installed_size(&staging)?;
    assert_eq!(size, 2); // (1024 + 512) / 1024 = 1.5 -> ceil = 2
    Ok(())
}
