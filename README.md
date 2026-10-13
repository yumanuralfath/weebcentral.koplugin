# WeebCentral Downloader for KOReader

KOReader plugin to search WeebCentral manga, select chapters, and download them as CBZ files. The KOReader interface is written in Lua; the downloader core is written in Rust.

## Features

- Search manga on WeebCentral from inside KOReader.
- List chapters and pick what to download.
- Chapter selection with comma-separated numbers and ranges, such as `1,3,4` or `1-10`.
- Output as CBZ files; giving a volume name combines the selection into one CBZ.
- Prebuilt releases for every [KOReader koxtoolchain](https://github.com/koreader/koxtoolchain) target (Kindle, Kobo, PocketBook, reMarkable, Cervantes, Bookeen).

## Install a release

1. Open the repository's **Releases** page.
2. Download the ZIP that matches your device (see [Choose your toolchain](#choose-your-toolchain)):
   `WeebCentralDownloader-<version>-<toolchain>.zip`
3. Optionally verify it against `SHA256SUMS`:

   ```
   sha256sum --check --ignore-missing SHA256SUMS
   ```

4. Extract the ZIP. It contains a `WeebCentral.koplugin` directory.
5. Copy that directory into KOReader's `plugins` directory on your device. On a typical Kindle this is `/mnt/us/koreader/plugins/`.
6. Restart KOReader.
7. Open **Tools > WeebCentral Downloader**.

Each ZIP contains only the plugin files and the executable built for that toolchain. The bundled KOReader source tree in this repository is for development and is not included in the install package.

## Choose your toolchain

The toolchain name follows the device's KOReader ABI, not just the model name. Pick the ZIP for your device from the table below. The device notes come from the [koxtoolchain README](https://github.com/koreader/koxtoolchain); check it if your device is not listed.

| Toolchain            | Devices (from koxtoolchain notes)        | Rust target                     | Status       |
| -------------------- | ---------------------------------------- | ------------------------------- | ------------ |
| `kindlehf`           | Any Kindle on firmware 5.16.3 or newer   | `armv7-unknown-linux-gnueabihf` | Stable       |
| `kindlepw2`          | Kindle Paperwhite 2 and everything since | `armv7-unknown-linux-gnueabi`   | Experimental |
| `kindle5`            | Kindle 4, Touch, Paperwhite 1            | `armv7-unknown-linux-gnueabi`   | Experimental |
| `kindle`             | Kindle 2, DX, DXg, 3                     | `arm-unknown-linux-gnueabi`     | Experimental |
| `kobo`               | Kobo (generic)                           | `armv7-unknown-linux-gnueabihf` | Stable       |
| `kobov4`             | Kobo Mark 7 class devices                | `armv7-unknown-linux-gnueabihf` | Stable       |
| `kobov5`             | Kobo and Tolino devices on firmware 5.x  | `armv7-unknown-linux-gnueabihf` | Stable       |
| `nickel`             | Kobo variant mimicking Kobo's own TC     | `armv7-unknown-linux-gnueabihf` | Stable       |
| `cervantes`          | Cervantes                                | `armv7-unknown-linux-gnueabi`   | Experimental |
| `pocketbook`         | PocketBook (max backward compatibility)  | `armv7-unknown-linux-gnueabi`   | Experimental |
| `pocketbookhf`       | PocketBook (hard-float)                  | `armv7-unknown-linux-gnueabihf` | Stable       |
| `remarkable`         | reMarkable, firmware 2.x or newer        | `armv7-unknown-linux-gnueabihf` | Stable       |
| `remarkable-aarch64` | reMarkable AArch64 devices               | `aarch64-unknown-linux-gnu`     | Stable       |
| `bookeen`            | Bookeen (tested on AWA13 devices only)   | `armv7-unknown-linux-gnueabi`   | Experimental |

**Status meaning**

- **Stable**: the release fails if this build fails.
- **Experimental**: soft-float or legacy-glibc targets. Rust's standard library expects a reasonably recent glibc, so these builds may fail or may not run on very old firmware. If one fails, the release still publishes without that ZIP. Rust's soft-float ARM targets also do not use the hardware FPU, which is fine for this plugin.

A mismatched ABI can produce a binary that builds fine but does not run on the device. If you are unsure, start with the toolchain KOReader itself uses for your device.

## Create a GitHub release

Releases are built only when manually triggered:

1. Push the source changes you want to release.
2. In GitHub, open **Actions > Build and release all KOReader toolchains**.
3. Choose **Run workflow**, enter a new version tag such as `v0.1.0`, and choose whether it is a pre-release.
4. Run the workflow.

What the workflow does ([`.github/workflows/release-all.yml`](.github/workflows/release-all.yml)):

1. Validates the tag and checks that the release does not already exist.
2. Runs the Rust tests once.
3. Builds the core in parallel for all 14 toolchains. For each one it downloads `<toolchain>.tar.zst` from [koxtoolchain 2026.08](https://github.com/koreader/koxtoolchain/releases/tag/2026.08), verifies its published SHA-256 before extraction, selects the matching Rust target, and cross-compiles.
4. Packages one `WeebCentralDownloader-<version>-<toolchain>.zip` per toolchain, each with a `.sha256` file.
5. Creates a single GitHub Release with every ZIP plus a combined `SHA256SUMS`.

The workflow is manual-only (`workflow_dispatch`). It does not build on pushes or pull requests. Each release tag must be new.

### Updating the toolchain version

When koxtoolchain publishes a new release:

1. Change `KOXTOOLCHAIN_VERSION` at the top of `release-all.yml`.
2. Update each `sha256` in the build matrix. The digests are listed next to each asset on the koxtoolchain release page.
3. Re-check the compiler prefixes if toolchains were added or renamed. The prefix is the directory created under `x-tools/` by each archive.

## Build locally

### Native development build

From the repository root:

```
cargo test --manifest-path weebcentral_core/Cargo.toml
cargo build --manifest-path weebcentral_core/Cargo.toml
cp weebcentral_core/target/debug/weebcentral_core \
  WeebCentral.koplugin/bin/weebcentral_core_dev
```

This native executable is for running on the build computer, not for installing on a device.

### Cross-compile with a koxtoolchain

Example for `kindlehf`. Download the archive from the [koxtoolchain release](https://github.com/koreader/koxtoolchain/releases/tag/2026.08) and extract it under `$HOME`. It creates `$HOME/x-tools/<compiler-prefix>`.

```
rustup target add armv7-unknown-linux-gnueabihf
export PATH="$HOME/x-tools/arm-kindlehf-linux-gnueabihf/bin:$PATH"

cd weebcentral_core
CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-kindlehf-linux-gnueabihf-gcc \
CC_armv7_unknown_linux_gnueabihf=arm-kindlehf-linux-gnueabihf-gcc \
AR_armv7_unknown_linux_gnueabihf=arm-kindlehf-linux-gnueabihf-ar \
cargo build --release --target armv7-unknown-linux-gnueabihf

install -D -m 755 \
  target/armv7-unknown-linux-gnueabihf/release/weebcentral_core \
  ../WeebCentral.koplugin/bin/weebcentral_core_dev
```

For another toolchain, change these together (taking values from the table above):

1. The archive you download and extract.
2. The `x-tools/<compiler-prefix>/bin` directory on `PATH`.
3. The Rust target passed to `rustup target add` and `--target`.
4. The target-specific variables, with the target's dashes turned into underscores:
   `CARGO_TARGET_<TARGET_UPPERCASE>_LINKER`, `CC_<target>` and `AR_<target>`.

The linker for `armv7-unknown-linux-gnueabihf` is also set in `weebcentral_core/.cargo/config.toml`. The explicit `CC` and `AR` variables are needed by native Rust dependencies such as `ring` when they compile C or assembly code for the target.

## Plugin layout

```
WeebCentral.koplugin/
├── _meta.lua
├── main.lua
├── lib/weebcentral/
│   ├── core.lua
│   └── ui.lua
└── bin/
    └── weebcentral_core_dev
```

The Rust core supports `search`, `chapters`, and `download` commands. Its JSON output is consumed by the Lua plugin.

## Repository layout

| Path                    | Purpose                                          |
| ----------------------- | ------------------------------------------------ |
| `WeebCentral.koplugin/` | KOReader plugin (Lua UI and bundled core binary) |
| `weebcentral_core/`     | Rust downloader core                             |
| `.github/workflows/`    | Release workflow for all koxtoolchain targets    |
