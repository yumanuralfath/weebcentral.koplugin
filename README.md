# YuManga for KOReader

KOReader plugin to search manga from multiple sources, select chapters, and download them as CBZ files. The KOReader interface is written in Lua; the downloader core is written in Rust.

Built-in sources currently include **WeebCentral** (as a Rhai extension). More sites can be added by dropping a `.rhai` script into the plugin — no core rebuild required, and the Lua UI picks them up automatically.

## Features

- Search manga across one or more selected sources from inside KOReader. Results show the cover, title, author, status, type, year and tags.
- Choose which sources to use from the plugin menu. New `.rhai` extensions appear automatically.
- Manga detail page with a large cover, status/type badges, tag chips, last update date and a scrollable synopsis.
- Chapter picker with checkboxes: tap to select, long-press to select a range, newest/oldest first, jump between pages.
- Quick actions: download the latest 5 chapters or the whole series.
- The extra search details can be switched off in the plugin menu if your connection is slow.
- Pick chapters by typing numbers too.
- Chapter selection with comma-separated numbers and ranges, such as `1,3,4` or `1-10`.
- Output as CBZ files; giving a volume name combines the selection into one CBZ.
- Prebuilt releases for every [KOReader koxtoolchain](https://github.com/koreader/koxtoolchain) target (Kindle, Kobo, PocketBook, reMarkable, Cervantes, Bookeen).

## Install a release

1. Open the repository's **Releases** page.
2. Download the ZIP that matches your device (see [Choose your toolchain](#choose-your-toolchain)):
   `YuManga-<version>-<toolchain>.zip`
3. Optionally verify it against `SHA256SUMS`:

   ```
   sha256sum --check --ignore-missing SHA256SUMS
   ```

4. Extract the ZIP. It contains a `YuManga.koplugin` directory.
5. Copy that directory into KOReader's `plugins` directory on your device. On a typical Kindle this is `/mnt/us/koreader/plugins/`.
6. Restart KOReader.
7. Open **Tools > YuManga**.

Each ZIP contains only the plugin files and the executable built for that toolchain. The bundled KOReader source tree in this repository is for development and is not included in the install package.

If you previously used **WeebCentral Downloader**, remove the old `WeebCentral.koplugin` folder after installing YuManga. Settings keys and cache paths are separate (`yumanga_*`).

## Choose your toolchain

The toolchain name follows the device's KOReader ABI, not just the model name. Pick the ZIP for your device from the table below. The device notes come from the [koxtoolchain README](https://github.com/koreader/koxtoolchain); check it if your device is not listed.

| Toolchain            | Devices (from koxtoolchain notes)        | Rust target                     | Status       |
| -------------------- | ---------------------------------------- | ------------------------------- | ------------ |
| `kindlehf`           | Any Kindle on firmware 5.16.3 or newer   | `armv7-unknown-linux-gnueabihf` | Stable       |
| `kindlepw2`          | Kindle Paperwhite 2 and everything since | `armv7-unknown-linux-gnueabi`   | Experimental |
| `kindle5`            | Kindle 4, Touch, Paperwhite 1            | `armv7-unknown-linux-gnueabi`   | Experimental |
| `kindle`             | Kindle 2, DX, DXg, 3                     | `arm-unknown-linux-gnueabi`     | Experimental |
| `kobo`               | Kobo (generic, older devices)            | `armv7-unknown-linux-gnueabihf` | Experimental |
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

- **Stable**: dynamically linked against the toolchain's glibc. The sysroot is glibc 2.17 or newer, which Rust's standard library and the `ring` TLS dependency require. The release fails if one of these builds fails.
- **Experimental**: the toolchain's sysroot is older than glibc 2.16 (`kindle` and `pocketbook` 2.9, `bookeen` 2.11, `kindle5`, `kindlepw2` and `cervantes` 2.12, `kobo` 2.15). Rust's standard library and the `ring` TLS dependency call `getauxval()`, which these sysroots do not have, so a plain build fails at link time with `undefined reference to 'getauxval'`. The workflow handles this in two steps:
  1. Build the normal dynamic glibc binary and link in a tiny `getauxval` shim that reads `/proc/self/auxv`.
  2. If that still fails (for example because another symbol is missing), fall back to a **statically linked musl** binary, using the koxtoolchain GCC only as C compiler and linker driver.

  The release notes list which variant each experimental ZIP ended up using. None of the experimental builds have been tested on real devices. If both attempts fail, the release still publishes without that ZIP.

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
3. Builds the core in parallel for all 14 toolchains. For each one it downloads `<toolchain>.tar.zst` from [koxtoolchain 2026.08](https://github.com/koreader/koxtoolchain/releases/tag/2026.08), verifies its published SHA-256 before extraction, selects the matching Rust target, and cross-compiles (with the `getauxval` shim and musl fallback for old sysroots, see above).
4. Packages one `YuManga-<version>-<toolchain>.zip` per toolchain, each with a `.sha256` file.
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
cargo test --manifest-path yumanga_core/Cargo.toml
cargo build --manifest-path yumanga_core/Cargo.toml
cp yumanga_core/target/debug/yumanga_core \
  YuManga.koplugin/bin/yumanga_core_dev
```

This native executable is for running on the build computer, not for installing on a device.

### Cross-compile with a koxtoolchain

Example for `kindlehf`. Download the archive from the [koxtoolchain release](https://github.com/koreader/koxtoolchain/releases/tag/2026.08) and extract it under `$HOME`. It creates `$HOME/x-tools/<compiler-prefix>`.

```
rustup target add armv7-unknown-linux-gnueabihf
export PATH="$HOME/x-tools/arm-kindlehf-linux-gnueabihf/bin:$PATH"

cd yumanga_core
CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-kindlehf-linux-gnueabihf-gcc \
CC_armv7_unknown_linux_gnueabihf=arm-kindlehf-linux-gnueabihf-gcc \
AR_armv7_unknown_linux_gnueabihf=arm-kindlehf-linux-gnueabihf-ar \
cargo build --release --target armv7-unknown-linux-gnueabihf

install -D -m 755 \
  target/armv7-unknown-linux-gnueabihf/release/yumanga_core \
  ../YuManga.koplugin/bin/yumanga_core_dev
```

For another toolchain, change these together (taking values from the table above):

1. The archive you download and extract.
2. The `x-tools/<compiler-prefix>/bin` directory on `PATH`.
3. The Rust target passed to `rustup target add` and `--target`.
4. The target-specific variables, with the target's dashes turned into underscores:
   `CARGO_TARGET_<TARGET_UPPERCASE>_LINKER`, `CC_<target>` and `AR_<target>`.

The linker for `armv7-unknown-linux-gnueabihf` is also set in `yumanga_core/.cargo/config.toml`. The explicit `CC` and `AR` variables are needed by native Rust dependencies such as `ring` when they compile C or assembly code for the target.

## Plugin layout

```
YuManga.koplugin/
├── _meta.lua
├── main.lua
├── extensions/
│   └── weebcentral.rhai
├── lib/yumanga/
│   ├── core.lua
│   └── ui.lua
└── bin/
    └── yumanga_core_dev
```

The Rust core supports `sources`, `search`, `chapters`, and `download` commands. Its JSON output is consumed by the Lua plugin.

## Add a manga source (no recompile)

Sources are Rhai scripts in `YuManga.koplugin/extensions/`. The core loads every `*.rhai` next to the plugin binary at runtime.

Each script must define:

| Function | Returns |
| -------- | ------- |
| `id()` | Stable id used on the CLI (e.g. `"weebcentral"`) |
| `name()` | Label shown in the UI |
| `search(keyword, cover_cache_dir, with_details)` | Array of result maps |
| `series(series_id)` | Series map; `chapters` oldest → newest |
| `pages(chapter_id, chapter_url)` | Array of `{ url, referer }` |

Host helpers available to scripts: `http_get`, `http_post_form`, `select_outer`, `select_attr`, `select_text`, `abs_url`, `id_from_href`, `normalize_ws`, `regex_captures`, `regex_find_ints`, `cache_cover`, `derive_page_urls`, `ends_with_colon_label`.

Copy a new `.rhai` into `extensions/` on the device (or into this repo before packaging), restart KOReader, and the source appears under **Tools > YuManga**. Only changes to the host API itself need a new `yumanga_core` build.

## Repository layout

| Path                 | Purpose                                          |
| -------------------- | ------------------------------------------------ |
| `YuManga.koplugin/`  | KOReader plugin (Lua UI and bundled core binary) |
| `yumanga_core/`      | Rust downloader core (Rhai host + download/CBZ)  |
| `YuManga.koplugin/extensions/` | Per-site Rhai scripts (no recompile to add) |
| `.github/workflows/` | Release workflow for all koxtoolchain targets    |
