# WeebCentral Downloader for KOReader

KOReader plugin to search WeebCentral manga, select chapters, and download them as CBZ files. The KOReader interface is written in Lua; the downloader core is written in Rust.

## Install a Kindle HF release

1. Open the repository's **Releases** page and download `WeebCentralDownloader-<version>-kindlehf.zip`.
2. Extract the ZIP. It contains a `WeebCentral.koplugin` directory.
3. Copy that directory into KOReader's `plugins` directory on the Kindle. For a typical Kindle installation, this is under `/mnt/us/koreader/plugins/`.
4. Restart KOReader.
5. Open **Tools > WeebCentral Downloader**.

The released ZIP contains only the plugin files and its Kindle HF executable. The bundled KOReader source tree in this repository is for development and is not included in the install package.

## Create a GitHub release

Releases are built only when manually triggered:

1. Push the source changes you want to release.
2. In GitHub, open **Actions > Build and release Kindle HF plugin**.
3. Choose **Run workflow**, enter a new version tag such as `v0.1.0`, and choose whether it is a pre-release.
4. Run the workflow. It tests the Rust core, downloads the pinned KOReader `kindlehf` toolchain, cross-compiles the binary, places it at `WeebCentral.koplugin/bin/weebcentral_core_dev`, packages the plugin, and attaches the ZIP and SHA-256 file to a GitHub Release.

The workflow is manual-only (`workflow_dispatch`). It does not build on pushes or pull requests. Each release tag must be new.

The workflow uses the `kindlehf.tar.zst` asset from [KOReader koxtoolchain 2026.08](https://github.com/koreader/koxtoolchain/releases/tag/2026.08) and verifies its published SHA-256 before extraction.

## Build locally

### Native development build

From the repository root:

```sh
cargo test --manifest-path weebcentral_core/Cargo.toml
cargo build --manifest-path weebcentral_core/Cargo.toml
cp weebcentral_core/target/debug/weebcentral_core \
  WeebCentral.koplugin/bin/weebcentral_core_dev
```

This native executable is for running on the build computer, not for installing on a Kindle.

### Cross-compile for Kindle HF

The configured Rust target and linker are in `weebcentral_core/.cargo/config.toml`:

- Rust target: `armv7-unknown-linux-gnueabihf`
- KOX compiler prefix: `arm-kindlehf-linux-gnueabihf`

Download `kindlehf.tar.zst` from the KOX 2026.08 release, then extract it under `$HOME`. The archive should create `$HOME/x-tools/arm-kindlehf-linux-gnueabihf`.

```sh
rustup target add armv7-unknown-linux-gnueabihf
export PATH="$HOME/x-tools/arm-kindlehf-linux-gnueabihf/bin:$PATH"

cd weebcentral_core
CC_armv7_unknown_linux_gnueabihf=arm-kindlehf-linux-gnueabihf-gcc \
AR_armv7_unknown_linux_gnueabihf=arm-kindlehf-linux-gnueabihf-ar \
cargo build --release --target armv7-unknown-linux-gnueabihf

install -D -m 755 \
  target/armv7-unknown-linux-gnueabihf/release/weebcentral_core \
  ../WeebCentral.koplugin/bin/weebcentral_core_dev
```

The linker is selected by `.cargo/config.toml`. The explicit `CC` and `AR` variables are needed by native Rust dependencies such as `ring` when they compile C/assembly code for the target.

## Build for another Kindle

The KOX toolchain name is based on the device's KOReader ABI, not simply the Kindle model name. Check the compatibility notes in the [KOX toolchain release](https://github.com/koreader/koxtoolchain/releases/tag/2026.08) before selecting one:

| KOX toolchain | KOReader device family from KOX notes                         | Compiler prefix                |
| ------------- | ------------------------------------------------------------- | ------------------------------ |
| `kindlehf`    | Any Kindle running firmware 5.16.3 or newer                   | `arm-kindlehf-linux-gnueabihf` |
| `kindlepw2`   | Kindle Paperwhite 2 and newer devices using that KOReader ABI | `arm-kindlepw2-linux-gnueabi`  |
| `kindle5`     | Kindle 4, Touch, and Paperwhite 1                             | `arm-kindle5-linux-gnueabi`    |
| `kindle`      | Kindle 2, DX, DXg, and Kindle 3                               | `arm-kindle-linux-gnueabi`     |

For another family, adapt all of these together; do not only change the compiler executable:

1. Download that family's archive, for example `kindle5.tar.zst`, and extract it under `$HOME`.
2. Set the matching `x-tools/<compiler-prefix>/bin` directory on `PATH`.
3. Select a Rust target whose architecture and float ABI match the KOX sysroot. Check available targets with `rustc --print target-list`; older ARM targets may require a Rust target not distributed by rustup or a custom `build-std` setup.
4. Change the target and linker in `weebcentral_core/.cargo/config.toml`.
5. Set target-specific `CC_<target>` and `AR_<target>` variables to that toolchain's compiler and archiver.
6. Build with `cargo build --release --target <rust-target>`.

For example, the `kindlepw2` KOX prefix is not compatible with the `kindlehf` hard-float Rust target. Confirm the target ABI against the KOReader device/toolchain; a mismatched float ABI can produce a binary that builds successfully but will not run on the Kindle.

## Plugin layout

```text
WeebCentral.koplugin/
├── _meta.lua
├── main.lua
├── lib/weebcentral/
│   ├── core.lua
│   └── ui.lua
└── bin/
    └── weebcentral_core_dev
```

The Rust core supports `search`, `chapters`, and `download` commands. Its JSON output is consumed by the Lua plugin. Download selection accepts comma-separated chapter numbers and ranges such as `1,3,4` or `1-10`; providing a volume name combines the selection into one CBZ.
