# `bamabi`

Layer 3 (**Bevy `0.20.1` companion crates & sole canonical host facade**, `#![forbid(unsafe_code)]`) for the federated **`bamabi`** (`ABI 0.4.0`, `v0.4.0`) native modding framework.

Refactored from the original [`bevy_mod_abi`](https://github.com/yaga-simha/bevy_mod_abi) monorepo into a strictly acyclic 5-repository DAG plus [`bamabi-dev-overlay`](https://github.com/yaga-simha/bamabi-dev-overlay), `bamabi` depends on Layer 0 ([`bamabi-wire`](https://github.com/yaga-simha/bamabi-wire)) and Layer 2 ([`bamabi-core`](https://github.com/yaga-simha/bamabi-core)) via GitHub `git` URLs (`branch = "main"`) with zero dependency on Layer 1 (`bamabi-tools`). It is the primary entry point for Bevy game hosts, re-exporting the slim headless `bamabi_host` runtime and wire/macro primitives alongside opt-in Bevy `0.20.1` subsystem bridges, render/material extraction, dynamic reflection/savegames/telemetry, and curated presets.

> **Note:** The legacy `bevy_abi_mod` compatibility shim from the pre-split monorepo was removed in `0.3.0`. `bamabi` is the sole canonical host facade crate.

## Workspace Crates

| Crate | Safety Policy | Responsibilities |
|---|---|---|
| [`bamabi`](crates/bamabi/README.md) | `#![forbid(unsafe_code)]` | Canonical top-level host facade re-exporting `bamabi_host`, derives, `bamabi_sys`, companion crates, 4 curated presets, 18 granular feature flags, and unified `prelude` |
| [`bamabi_bridges`](crates/bamabi_bridges/README.md) | `#![forbid(unsafe_code)]` | 12 opt-in Bevy `0.20.1` subsystem bridges (`default = []`: `std-types`, `state`, `asset`, `bsn`, `picking`, `camera-2d`, `light`, `camera-controller`, `solari`, `audio`, `ui`, `feathers`) + 4 curated presets |
| [`bamabi_render`](crates/bamabi_render/README.md) | `#![forbid(unsafe_code)]` | `ModSubAppPlugin<L>`, `.wesl` 2D/3D material & `ShaderBuffer` modding, opt-in `ModMeshShaderPlugin`, and bounded `DeviceLost` recovery (`ModRenderRecoveryPlugin`) |
| [`bamabi_reflect`](crates/bamabi_reflect/README.md) | `#![forbid(unsafe_code)]` | `ModReflectPlugin` (witness & declarative `BamFieldDesc` `ReflectFromPtr`/BRP/`bevy-settings`), `ModSaveGamePlugin` (`HandleSerializeProcessor`/`HandleDeserializeProcessor`), and `ModTelemetryPlugin` |

## Standalone Usage

`bamabi` builds and runs completely standalone with zero sibling checkouts required.

Downstream Bevy `0.20.1` applications add `bamabi` in `Cargo.toml` directly via its GitHub `git` URL:

```toml
[dependencies]
# Slim headless host core (bevy_ecs + bevy_app only)
bamabi = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main" }

# 2D game preset + filesystem hot-reload watcher:
# bamabi = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main", features = ["preset-2d", "hot-reload-watcher"] }

# 3D game preset + SubApp render extraction + reflection/savegames/telemetry + signature verification:
# bamabi = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main", features = ["preset-3d", "sub-app-extract", "bridge-reflect", "signature-verification"] }
```

See [`crates/bamabi/README.md`](crates/bamabi/README.md) for the Quickstart example and complete 18-flag reference.

## Curated Presets

| Preset Feature | Bundle Plugin | Included Bridge Features |
|---|---|---|
| `preset-2d` | `ModPreset2dPlugin` | `bridge-std-types`, `bridge-asset`, `bridge-state`, `bridge-picking`, `bridge-audio`, `bridge-camera-2d`, `bridge-ui` |
| `preset-3d` | `ModPreset3dPlugin` | `preset-2d`, `bridge-light`, `bridge-camera-controller`, `bridge-bsn` |
| `preset-ui` | `ModPresetUiPlugin` | `bridge-std-types`, `bridge-asset`, `bridge-picking`, `bridge-ui`, `bridge-feathers` |
| `preset-full` | `ModPresetFullPlugin` | `preset-3d`, `preset-ui`, `bridge-solari` |

## Local & CI Verification

All checks run standalone inside `bamabi` with zero sibling repositories required. On Linux, `bridge-audio` (`alsa-sys` / `rodio`) requires ALSA development headers (`sudo apt-get install -y libasound2-dev pkg-config`):

```bash
# Formatting, #![forbid(unsafe_code)], and Bevy idiom audit
cargo fmt --all -- --check
for f in crates/*/src/lib.rs; do grep -q '#!\[forbid(unsafe_code)\]' "$f"; done
grep -rn '&mut ResMut<' crates/*/src   # must print nothing

# Clippy (default, signature-verification, all-features) and Rustdoc (default & all-features)
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p bamabi --features signature-verification --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# Feature isolation checks (each grep must print nothing) and preset matrix
cargo tree --edges normal -p bamabi_bridges --no-default-features | sed 's/ (.*//' | grep -E 'bevy_render|bevy_pbr|bevy_light|bevy_feathers|bevy_solari'
cargo tree --edges normal -p bamabi --no-default-features --features preset-2d | sed 's/ (.*//' | grep -E 'bevy_pbr|bevy_light|bevy_solari|bevy_camera_controller'
cargo tree --edges normal -p bamabi --no-default-features | sed 's/ (.*//' | grep -E 'bevy_render|bevy_reflect|bevy_scene|bevy_asset|\bnotify\b'
cargo check -p bamabi --no-default-features --features preset-2d
cargo check -p bamabi --no-default-features --features preset-3d
cargo check -p bamabi --no-default-features --features preset-ui
cargo check -p bamabi --no-default-features --features preset-full
cargo check -p bamabi --no-default-features --features bridge-camera-2d
cargo check -p bamabi_bridges --no-default-features --features bridge-camera-2d

# Test suites (default, signature-verification, all-features, doctests) and coverage
cargo test --workspace
cargo test -p bamabi --features signature-verification
cargo test --workspace --all-features
cargo test --doc --all-features
cargo llvm-cov --no-report --workspace --all-features
cargo llvm-cov report --ignore-filename-regex '(tests|examples|benches)/|\.cargo/' --summary-only

# Spellcheck and dependency/license/source audit
typos
cargo deny --all-features check advisories licenses bans sources
```

> **Environment Overrides (`BAM_WIRE_DIR`, `BAM_CORE_DIR`, `BAM_FIXTURE_DIR`):** By default in standalone checkouts, `bridges_integration` builds temporary `cdylib` guest crates against `https://github.com/yaga-simha/bamabi-wire.git`, and `bamabi_render` tests render `bamabi.h` via `bamabi_sdk_gen` with an inline C99 fallback. Set `BAM_WIRE_DIR`, `BAM_CORE_DIR`, or `BAM_FIXTURE_DIR` to point tests at local sibling checkouts or prebuilt fixtures.

## Federated `bamabi` Repositories

| Layer | Repository | Role |
|---|---|---|
| **0** | [`bamabi-wire`](https://github.com/yaga-simha/bamabi-wire) | Zero-Bevy ABI wire contract (`spec.md`), derives, `include/bamabi.h`, and Rust guest SDK |
| **1** | [`bamabi-tools`](https://github.com/yaga-simha/bamabi-tools) | Zero-Bevy Ed25519 trust (`bamabi_trust`), manifest DAG (`bamabi_manifest`), and `cargo-bam` CLI |
| **2** | [`bamabi-core`](https://github.com/yaga-simha/bamabi-core) | Headless `bevy_ecs` + `bevy_app` host runtime (`bamabi_host_sys`, `bamabi_host`, `bamabi_watcher`) and `docs/ARCHITECTURE.md` |
| **3** | **[`bamabi`](https://github.com/yaga-simha/bamabi)** *(this repo)* | Bevy `0.20.1` companion bridges/render/reflect crates and sole canonical `bamabi` host facade |
| **4** | [`bamabi-showcase`](https://github.com/yaga-simha/bamabi-showcase) | Top-level integration repo: runnable examples (`01`–`10`), benchmarks (`docs/BENCHMARKS.md`), fuzzing, and mdBook guide (`docs/book/`) |
| **Overlay** | [`bamabi-dev-overlay`](https://github.com/yaga-simha/bamabi-dev-overlay) | Multi-repo `abi-lock.toml` / standalone-git consistency verifier (`sync_check.py`) and local sibling `[patch]` overlay (`overlay.py`) |

## Multi-Repo Local Development (`bamabi-dev-overlay`)

When iterating on `bamabi` alongside sibling checkouts (`bamabi-wire`, `bamabi-tools`, `bamabi-core`, `bamabi-showcase`) under a common parent directory, use [`bamabi-dev-overlay`](https://github.com/yaga-simha/bamabi-dev-overlay) instead of editing tracked `Cargo.toml` dependencies:

```bash
# Verify ABI lockfiles, versions, licenses, and strict GitHub dependency hygiene across all repos
python3 ../bamabi-dev-overlay/sync_check.py --strict-git

# Temporarily patch upstream github.com/yaga-simha/*.git deps to local sibling paths
python3 ../bamabi-dev-overlay/overlay.py enable
python3 ../bamabi-dev-overlay/overlay.py status --strict

# Disable the local overlay before committing or testing standalone builds (Cargo.lock hygiene)
python3 ../bamabi-dev-overlay/overlay.py disable
```

> **Note (`Cargo.lock` hygiene):** Never commit `Cargo.lock` changes generated while `[patch]` is enabled; tracked `Cargo.lock` must retain `git+https://github.com/yaga-simha/bamabi-{wire,tools,core}.git?branch=main#...` sources for standalone CI.

## License

Dual-licensed under `MIT OR Apache-2.0` ([`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE)).
