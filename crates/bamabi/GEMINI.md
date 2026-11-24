# Agent Guide: `bamabi`

Crate guide for **`bamabi`** (`v0.4.0`, ABI `0x0004_0000`, Bevy `0.20.1+`) — the **canonical** top-level Bevy host facade crate.

---

## 1. Crate Identity
- **Name:** `bamabi` (`version = "0.4.0"`, `edition = "2024"`, MSRV `1.95`)
- **Role:** Canonical top-level Bevy host facade unifying `bamabi_host`, `bamabi_macros`, `bamabi_sys`, and opt-in companion crates (`bridges`, `render`, `reflect`, `watcher`) behind 18 granular feature flags, 4 curated bridge presets, and `prelude`.
- **Retired Shim Note:** The legacy `bevy_abi_mod` forwarding shim was retired in `0.3.0`; `bamabi` is the sole canonical host facade crate.

---

## 2. Safety & Dependency Boundaries
- **Safety:** **`#![forbid(unsafe_code)]`** at crate root. Never introduce `unsafe` code here.
- **Default Headless Ergonomics:** Default features MUST stay lean — enabling NO `bevy_render`, `bevy_reflect`, `bevy_scene`, `bevy_asset`, or `notify` unless explicitly opted in via feature flags.

---

## 3. Owned Surface & 22 Feature Flags
- **Core Re-exports (`src/lib.rs`, `prelude`):** `AbiModPlugin`, `ModAppExt`, `ModWorldExt`, `ModCommandsExt`, `ModComponent`, `AbiPod`, `GuestComponent`, `ComponentOptions`, `ModBudgetPolicy`, `ModUnloadHooks`, and `bamabi_sys` wire primitives (`sys`).
- **Core Features:** `strict-ffi-checks`, `diagnostics`, `signature-verification`, `hot-reload-watcher` (`bamabi_watcher`).
- **Curated Bridge Presets (mirrored 1:1 from `bamabi_bridges`):**
  - `preset-2d` (`ModPreset2dPlugin`): `bridge-std-types`, `bridge-asset`, `bridge-state`, `bridge-picking`, `bridge-audio`, `bridge-camera-2d`, `bridge-ui`
  - `preset-3d` (`ModPreset3dPlugin`): `preset-2d`, `bridge-light`, `bridge-camera-controller`, `bridge-bsn`
  - `preset-ui` (`ModPresetUiPlugin`): `bridge-std-types`, `bridge-asset`, `bridge-picking`, `bridge-ui`, `bridge-feathers`
  - `preset-full` (`ModPresetFullPlugin`): `preset-3d`, `preset-ui`, `bridge-solari`
- **Granular Companion Features:** `bridge-std-types`, `bridge-asset`, `bridge-state`, `bridge-bsn`, `bridge-picking`, `bridge-audio`, `bridge-camera-2d`, `bridge-light` (implies `bridge-camera-2d`), `bridge-ui`, `bridge-feathers` (implies `bridge-ui`), `bridge-camera-controller`, `bridge-solari`, `sub-app-extract` (`bamabi_render` + `ModRenderRecoveryPlugin`), `bridge-reflect` (`bamabi_reflect` + `EphemeralHandleBehavior`).

---

## 4. Strict Engineering Rules
1. **1:1 Preset & Export Parity:** Whenever a feature or public type is added or modified in `bamabi_bridges`, `bamabi_render`, `bamabi_reflect`, or `bamabi_watcher`, mirror its feature gate and re-export (`root`, module, and `prelude`) here.
2. **Facade Thinness:** Keep this crate purely a re-export and integration-test facade; place implementation logic in `bamabi_host` or the respective companion crate.
3. **Canonical Facade Imports:** Tests and doctests here MUST import `bamabi::*`.

---

## 5. Standalone Verification Commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo check --no-default-features --features preset-2d
cargo check --no-default-features --features preset-3d
cargo check --no-default-features --features preset-ui
cargo check --no-default-features --features preset-full
cargo tree -e no-dev --no-default-features --features preset-2d | grep -E "bevy_pbr|bevy_light|bevy_solari|bevy_camera_controller"   # MUST print NOTHING
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
typos
```

---

## 6. Workspace Coordination
1. **Standalone `bamabi` Layer 3 Member:** `bamabi` is a Layer 3 workspace member whose shared dependencies and lints live in `../../Cargo.toml` (resolving Layer 0 `bamabi-wire` and Layer 2 `bamabi-core` via GitHub git URLs, with optional local sibling overrides via `bamabi-dev-overlay/overlay.py`).
2. **Self-Contained Integration Tests:** `tests/bridges_integration.rs` honors `BAM_WIRE_DIR` or sibling `../bamabi-wire` when present and falls back automatically to `https://github.com/yaga-simha/bamabi-wire.git` (`branch = "main"`) in standalone checkouts.
3. **Preset Sync Check:** Ensure `Cargo.toml` feature lists for `preset-2d`, `preset-3d`, `preset-ui`, and `preset-full` match `../bamabi_bridges/Cargo.toml` 1:1 (including activating matching `bridge-*` flags for `#[cfg(feature = "...")]` exports).
