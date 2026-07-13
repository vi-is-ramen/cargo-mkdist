use crate::stage::*;
use std::fs::{self, File};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

#[test]
fn copy_binary_works()
{
    let staging = Staging::new().unwrap();
    let target_dir = PathBuf::from("target")
        .join("x86_64-unknown-linux-gnu")
        .join("release");
    fs::create_dir_all(&target_dir).unwrap();
    let bin_path = target_dir.join("myapp");
    File::create(&bin_path)
        .unwrap()
        .write_all(b"fake binary")
        .unwrap();
    // Сделаем исполняемым, если Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).unwrap();
    }

    let dest = staging
        .copy_binary("myapp", "x86_64-unknown-linux-gnu", false)
        .unwrap();
    assert!(dest.exists());
    let dest_content = fs::read_to_string(&dest).unwrap();
    assert_eq!(dest_content, "fake binary");
    #[cfg(unix)]
    {
        let meta = fs::metadata(&dest).unwrap();
        let mode = meta.permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }
    // cleanup
    fs::remove_dir_all("target").ok();
}

#[test]
fn copy_extra_files_works()
{
    let staging = Staging::new().unwrap();
    let src_file = staging.root.path().join("src.txt");
    File::create(&src_file)
        .unwrap()
        .write_all(b"extra")
        .unwrap();
    let files = vec![(
        src_file.to_str().unwrap().to_string(),
        "etc/myapp/extra.txt".to_string(),
        "644".to_string(),
    )];
    staging.copy_extra_files(&files).unwrap();
    let dest = staging.root.path().join("etc/myapp/extra.txt");
    assert!(dest.exists());
    #[cfg(unix)]
    {
        let meta = fs::metadata(&dest).unwrap();
        let mode = meta.permissions().mode();
        assert_eq!(mode & 0o777, 0o644);
    }
}
