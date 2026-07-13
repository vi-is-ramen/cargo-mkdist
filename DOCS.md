# cargo mkdist – full documentation

## Introduction

`cargo mkdist` is a Cargo subcommand that builds native distribution packages
from Rust projects – **without calling external packaging tools** like
`dpkg-deb`, `rpmbuild`, or `makepkg`.

It reads your `Cargo.toml` and a simple `dist.toml` configuration, builds your
binary (or workspace package) for any Rust target triple, collects files into
a staging area, and packs them into the requested format using pure Rust
libraries (`ar`, `tar`, `flate2`, `zstd`).

The result is a self‑contained packaging tool that works everywhere Rust runs.

---

## Installation

```bash
cargo install cargo-mkdist
```

After installation, the command `cargo mkdist` becomes available.

---

## Command‑line usage

```bash
cargo mkdist [OPTIONS] [TARGET] [-- <CARGO_BUILD_ARGS>...]
```

### Options

| Flag              | Description                                               |
| ----------------- | --------------------------------------------------------- |
| `--list`          | List all available targets defined in the configuration.  |
| `--all`           | Build all targets defined in the configuration.           |
| `--debug`         | Build in debug mode (default is release).                 |
| `--out-dir <DIR>` | Set output directory (default: `target/packages`).        |
| `--help`          | Show help.                                                |
| `--version`       | Show version.                                             |

If `TARGET` is given, only that target is built.
If no target is given and `--all` is not used, the target named `default` is used (if present).

All arguments after `--` are passed directly to `cargo build`.

### Examples

```bash
# Build the default target
cargo mkdist

# Build a specific target
cargo mkdist deb

# Build all targets
cargo mkdist --all

# Build with debug symbols and extra features
cargo mkdist arch -- --features=foo

# List available targets
cargo mkdist --list
```

---

## Configuration

Configuration is stored in a file named `dist-targets.toml` or `.distinfo/targets.toml`
(searched in the current directory, `.distinfo/`, or `.cargo/`).

The file must contain a `[targets]` table where each key is a target name and the value is a `TargetConfig`.

### Target fields

| Field         | Type              | Required? | Description                                                                               |
| ------------- | ----------------- | --------- | ----------------------------------------------------------------------------------------- |
| `target`      | string            | no        | Rust target triple (e.g. `x86_64-unknown-linux-gnu`). If omitted, must be inherited.      |
| `format`      | string            | yes       | Package format: `deb`, `pkg`, `tgz`.                                                      |
| `inherit`     | string            | no        | Name of another target to inherit fields from.                                            |
| `binaries`    | list of strings   | no        | Which binaries to include. If omitted, all `[[bin]]` targets from `Cargo.toml` are used.  |
| `package`     | string            | no        | Name of the workspace package to build (if not the root package).                         |
| `flags`       | string            | no        | Additional flags to pass to `cargo build` (e.g. `--features=foo`).                        |
| `extra`       | table             | no        | Format‑specific fields (see below).                                                       |

### Inheritance example

```toml
[targets.linux-base]
target = "x86_64-unknown-linux-gnu"
flags = "--release"
binaries = ["myapp"]

[targets.deb]
inherit = "linux-base"
format = "deb"
extra = { section = "utils" }

[targets.arch]
inherit = "linux-base"
format = "pkg"
extra = { license = "MIT" }
```

Here both `deb` and `arch` inherit `target`, `flags`, and `binaries` from `linux-base`.

---

## Format‑specific extra fields

### `deb` – Debian package

The following fields can be placed inside `extra`:

| Field                                 | Type      | Description                                                                   |
| ------------------------------------- | --------- | ----------------------------------------------------------------------------- |
| `section`                             | string    | e.g. `utils`, `admin`, `net` (default: `utils`)                               |
| `priority`                            | string    | `optional`, `required`, `important`, `standard` (default: `optional`)         |
| `maintainer`                          | string    | Override maintainer (default: from `authors` in `Cargo.toml`)                 |
| `description`                         | string    | Override package description (default: from `Cargo.toml`)                     |
| `depends`                             | string    | Comma‑separated list of dependencies, e.g. `libc6 (>= 2.34), libssl3`         |
| `recommends`                          | string    | Comma‑separated recommended packages                                          |
| `suggests`                            | string    | Comma‑separated suggested packages                                            |
| `conflicts`                           | string    | Comma‑separated conflicting packages                                          |
| `breaks`                              | string    | Comma‑separated packages this version breaks                                  |
| `replaces`                            | string    | Comma‑separated packages this replaces                                        |
| `provides`                            | string    | Comma‑separated virtual packages this provides                                |
| `enhances`                            | string    | Comma‑separated packages this enhances                                        |
| `pre-depends`                         | string    | Comma‑separated pre‑dependencies                                              |
| `conffiles`                           | string    | Comma‑separated list of configuration files (e.g. `/etc/myapp/config.toml`)   |
| `preinst`,`postinst`,`prerm`,`postrm` | string    | Paths to maintainer scripts (relative to project root)                        |

---

### `pkg` – Arch Linux package (`.pkg.tar.zst`)

| Field         | Type      | Description                                               |
| ------------- | --------- | --------------------------------------------------------- |
| `pkgrel`      | integer   | Package release number (default: 1)                       |
| `epoch`       | integer   | Epoch (optional)                                          |
| `license`     | string    | License string (e.g. `MIT`)                               |
| `url`         | string    | Upstream URL                                              |
| `depends`     | string    | Comma‑separated runtime dependencies                      |
| `optdepends`  | string    | Comma‑separated optional dependencies (with description)  |
| `conflicts`   | string    | Comma‑separated conflicting packages                      |
| `provides`    | string    | Comma‑separated virtual packages provided                 |
| `replaces`    | string    | Comma‑separated packages replaced                         |
| `backup`      | string    | Comma‑separated configuration files (relative to root)    |
| `install`     | string    | Path to `.INSTALL` script (relative to project root)      |

---

### `tgz` – simple tarball

No extra fields. The entire staging directory is packed into a `.tar.gz` archive.

---

## How it works (internally)

1. **Read configuration** – loads `dist.toml` and resolves inheritance.
2. **Build** – runs `cargo build --target <target>` with the given flags and optional `--package`.
3. **Prepare staging** – creates a temporary directory.
4. **Copy binaries** – copies the specified binaries from `target/<target>/<debug|release>/` into `usr/bin/` inside staging.
5. **Add extra files** – (if configured) copies additional files from the project into staging.
6. **Generate metadata** – for `deb`, creates `DEBIAN/control`, `conffiles`, and scripts; for `pkg`, creates `.PKGINFO` and copies `.INSTALL`.
7. **Pack** – produces the final package:
   - `deb`: builds `control.tar.gz` and `data.tar.gz`, then wraps them in an `ar` archive.
   - `pkg`: builds a `.pkg.tar.zst` archive containing all staging files plus metadata.
   - `tgz`: simply archives the staging directory.
8. **Output** – writes the package to `target/packages/<target-name>/`.

---

## Examples

### Minimal configuration for a single binary

```toml
[targets.default]
target = "x86_64-unknown-linux-gnu"
format = "deb"
binaries = ["myapp"]
extra = {
    depends = "libc6 (>= 2.34)",
    section = "utils"
}
```

### Multiple targets with inheritance

```toml
[targets.base]
target = "x86_64-unknown-linux-gnu"
flags = "--release"
binaries = ["myapp"]

[targets.deb]
inherit = "base"
format = "deb"
extra = {
    depends = "libc6 (>= 2.34), libssl3",
    conffiles = "/etc/myapp/config.toml",
    postinst = "scripts/postinst.sh"
}

[targets.arch]
inherit = "base"
format = "pkg"
extra = {
    pkgrel = 1,
    depends = "glibc, openssl",
    license = "MIT",
    install = "scripts/arch-install.sh"
}
```

### Building for a different architecture (cross‑compilation)

```toml
[targets.arm-deb]
target = "aarch64-unknown-linux-gnu"
format = "deb"
binaries = ["myapp"]
extra = {
    depends = "libc6 (>= 2.34)"
}
```

You must have the appropriate Rust target installed (`rustup target add aarch64-unknown-linux-gnu`) and cross‑compilation toolchain available.

---

## Limitations / known issues

- Currently only supports `deb`, `pkg.tar.zst`, and `tgz`. RPM and MSI are planned.
- For `deb`, the `Installed-Size` is computed as total size of files in KiB, rounded up – this is correct according to Debian policy.
- For `pkg`, the `size` field in `.PKGINFO` is the installed size; `csize` is not written (pacman ignores it).
- Permissions are preserved on Unix (file modes, symlinks). Windows support for packaging is not yet fully tested.
- Scripts and conffiles are only supported for `deb`; `pkg` supports `.INSTALL`.

---

## Contributing

Contributions are welcome! Feel free to open issues or pull requests on GitHub.

To build from source:

```bash
git clone https://github.com/yourname/cargo-mkdist
cd cargo-mkdist
cargo build --release
```

Run tests:

```bash
cargo test
```

---

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
