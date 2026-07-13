use crate::meta::*;
use cargo_metadata::camino::Utf8PathBuf;
use cargo_metadata::*;
use std::str::FromStr as _;

fn dummy_metadata() -> Metadata
{
    let root_id = PackageId {
        repr: "root 0.1.0 (path+file:///fake)".to_string(),
    };
    // let pkgb = PackageBuilder::new("root", "0.1.0", root_id, "file:///fake");
    let mut pkgb = PackageBuilder::default();

    let t1 = TargetBuilder::default()
        .name("mybin")
        .kind([TargetKind::Bin])
        .src_path("file:///fake")
        .build()
        .unwrap();

    let t2 = TargetBuilder::default()
        .name("otherbin")
        .kind([TargetKind::Bin])
        .src_path("file:///fake")
        .build()
        .unwrap();

    pkgb = pkgb.targets(vec![t1, t2]);

    pkgb = pkgb.name(PackageName::from_str("root").unwrap());
    pkgb = pkgb.version(semver::Version::from_str("0.1.0").unwrap());
    pkgb = pkgb.id(root_id.clone());
    pkgb = pkgb.manifest_path("file:///fake");

    let pkg = pkgb.build().unwrap();

    let res = ResolveBuilder::default()
        .nodes([])
        .root(Some(root_id.clone()))
        .build()
        .unwrap();

    MetadataBuilder::default()
        .packages(vec![pkg])
        .workspace_members(vec![root_id.clone()])
        .workspace_default_members(WorkspaceDefaultMembers::default())
        .resolve(Some(res))
        .workspace_root("/fake")
        .target_directory("/fake/target")
        .build_directory(Utf8PathBuf::from_str("/fake/target/build").unwrap())
        .workspace_metadata(0)
        .version(3usize)
        .build()
        .unwrap()
}

#[test]
fn find_package_by_name()
{
    let meta = dummy_metadata();
    let pkg = find_package(&meta, Some("root".to_string())).unwrap();
    assert_eq!(pkg.name, "root");
}

#[test]
fn find_package_root()
{
    let meta = dummy_metadata();
    let pkg = find_package(&meta, None).unwrap();
    assert_eq!(pkg.name, "root");
}

#[test]
fn find_package_not_found()
{
    let meta = dummy_metadata();
    let err = find_package(&meta, Some("unknown".to_string())).unwrap_err();
    assert!(err.to_string().contains("not found in workspace"));
}

#[test]
fn get_binary_names_works()
{
    let meta = dummy_metadata();
    let pkg = find_package(&meta, None).unwrap();
    let bins: Vec<String> = pkg
        .targets
        .iter()
        .filter(|t| t.kind.contains(&TargetKind::Bin))
        .map(|t| t.name.clone())
        .collect();
    assert_eq!(bins, vec!["mybin".to_string(), "otherbin".to_string()]);
}
