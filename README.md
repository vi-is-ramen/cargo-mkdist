# cargo mkdist

**Build native packages for your Rust projects – without system packaging tools.**

`cargo mkdist` is a Cargo subcommand that packages your Rust binary into `.deb`, `.pkg.tar.zst`, `.tar.gz` and more – **directly from Rust code**, with no dependency on `dpkg-deb`, `rpmbuild`, or `pacman`.

## How it works

1. Read your `Cargo.toml` and a simple `dist.toml` config.
2. Build your project (or workspace package) for any Rust target triple.
3. Copy binaries and extra files into a temporary staging directory.
4. Generate metadata (control files, `.PKGINFO`, etc.) and pack everything into the target format using pure Rust libraries (`ar`, `tar`, `flate2`, `zstd`).

No external tools are called – the whole packaging pipeline runs in-process.

## Why use it?

- **No extra dependencies** – works on any system where Rust runs.
- **Cross‑platform** – target any architecture with `--target`, package for Debian, Arch, or just a tarball.
- **Workspace‑aware** – pick specific packages from a workspace.
- **Simple config** – define targets with inheritance and format‑specific options in TOML.
- **Fast and reliable** – leverages Cargo’s metadata and Rust’s safety.

## Limitations (vs classic tools)

- Fewer formats (currently `deb`, `pkg.tar.zst`, `tgz` – RPM, MSI, APK planned).
- No built‑in repository signing or publishing (yet).
- May not support every niche Debian/Arch policy field – but covers common cases.

## Quick start

```bash
cargo install cargo-mkdist
cargo mkdist --list
cargo mkdist deb
```

See the [documentation](DOCS.md) to get started.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))

- MIT license ([LICENSE-MIT](./LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Feel free to open issues or pull requests.

---

Made with ❤️ for Rust community
