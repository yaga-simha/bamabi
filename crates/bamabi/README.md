# `bamabi`

Canonical top-level Bevy ECS (`v0.20.1`) host facade crate for the zero-serialization, asymmetric C-ABI (`ABI 0.3.0`, `v0.3.0`) native modding framework (Layer 3 in [`bamabi`](../../README.md)).

`bamabi` re-exports the slim headless `bamabi_host` (in [`bamabi-core`](https://github.com/yaga-simha/bamabi-core)), procedural derive macros (`#[derive(ModComponent, AbiPod, GuestComponent)]`) and `bamabi_sys` (in [`bamabi-wire`](https://github.com/yaga-simha/bamabi-wire)), optional companion crates ([`bamabi_bridges`](../bamabi_bridges), [`bamabi_render`](../bamabi_render), [`bamabi_reflect`](../bamabi_reflect), and `bamabi_watcher` in [`bamabi-core`](https://github.com/yaga-simha/bamabi-core)), curated bridge presets, and unified `prelude`, while enforcing `#![forbid(unsafe_code)]` across all public host APIs.

> **Note:** The legacy `bevy_abi_mod` compatibility shim from the pre-split monorepo was removed in `0.3.0`. `bamabi` is the sole canonical host facade crate.

## Installation

Add `bamabi` to your `Cargo.toml` directly from GitHub (`branch = "main"`):

```toml
[dependencies]
# Headless ECS core only (bevy_ecs + bevy_app)
bamabi = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main" }

# Or enable a curated bridge preset (preset-2d, preset-3d, preset-ui, or preset-full):
# bamabi = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main", features = ["preset-2d", "hot-reload-watcher"] }
# bamabi = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main", features = ["preset-3d", "sub-app-extract", "bridge-reflect"] }
```

## Quickstart

```rust,no_run
use bevy_app::App;
use bevy_ecs::component::Component;
use bamabi::prelude::*;

#[repr(C)]
#[derive(Component, ModComponent, Clone, Copy, Default)]
#[mod_component(name = "my_game::Health")]
struct Health {
    current: f32,
    max: f32,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(AbiModPlugin::with_error_policy(ModErrorPolicy::Quarantine));
    app.register_mod_component::<Health>(
        ComponentOptions::new().with_raw_insert().with_default(),
    )
    .expect("register Health");
    app.load_mod_with("mods/libcombat_mod.dylib", LoadOptions::default())
        .expect("load mod");
    app.run();
}
```

## Curated Bridge Presets

| Preset Feature | Bundle Plugin | Included Bridge Features |
|---|---|---|
| `preset-2d` | `ModPreset2dPlugin` | `bridge-std-types`, `bridge-asset`, `bridge-state`, `bridge-picking`, `bridge-audio`, `bridge-camera-2d`, `bridge-ui` |
| `preset-3d` | `ModPreset3dPlugin` | `preset-2d`, `bridge-light`, `bridge-camera-controller`, `bridge-bsn` |
| `preset-ui` | `ModPresetUiPlugin` | `bridge-std-types`, `bridge-asset`, `bridge-picking`, `bridge-ui`, `bridge-feathers` |
| `preset-full` | `ModPresetFullPlugin` | `preset-3d`, `preset-ui`, `bridge-solari` |

## Granular Feature Flags (18 Opt-In Flags)

| Feature | Purpose |
|---|---|
| `strict-ffi-checks` | Extra runtime FFI pointer, alignment, and handle-generation validation |
| `diagnostics` | Per-mod and aggregate `bevy_diagnostic` metrics (including budget telemetry) |
| `signature-verification` | Pure-Rust OpenSSH `SSHSIG`, Ed25519 CA, authority-scoped `SSHKRL`, `.bam_sig`, and hardened `.bampack` (`VerifiedBamPack::extract_to_dir`) pre-load verification |
| `hot-reload-watcher` | Debounced filesystem hot-reload watcher (`ModHotReloadPlugin`) |
| `bridge-std-types` | `Transform`, `GlobalTransform`, `Visibility`, `Name`, `Disabled`, `Time<*>`, and `ButtonInput<*>` bridges |
| `bridge-asset` | Sandboxed `mod://<namespace>/...` `AssetSource` & `AssetOverlayPolicy` (`ModAssetPlugin`) |
| `bridge-state` | Dynamic Bevy `States`, `SubStates`, and `ComputedStates` slabs (`ModStateExt`) |
| `bridge-bsn` | Bevy Scene Notation (`bevy_scene`) spawning (`ModBsnPlugin`, implies `bridge-asset`) |
| `bridge-picking` | Bevy `0.20` flattened `Pointer*` observers, `EntityCursor`, and `Gamepad` + rumble bridges |
| `bridge-audio` | Spatial `AudioPlayer<AudioSource>` and `PlaybackSettings` bridges (`ModAudioBridgePlugin`, implies `bridge-asset`) |
| `bridge-camera-2d` | Lightweight `Camera2d` bridge (`FfiCamera2d`) without pulling `bevy_pbr` or `bevy_light` |
| `bridge-light` | `Camera2d`/`Camera3d`, tonemapping, and `PointLight`/`SpotLight`/`DirectionalLight` bridges (implies `bridge-camera-2d`) |
| `bridge-camera-controller` | `PanOrbitCamera` controller bridge (`FfiPanOrbitCamera`) |
| `bridge-solari` | `SolariLighting` realtime GI bridge (`FfiSolariLighting`) |
| `bridge-ui` | Retained Bevy `0.20` UI, `EditableText`/`TextInput`, `InlineBox`/`InlineImage`, and headless widgets |
| `bridge-feathers` | Real `bevy_feathers` BSN-hydrated controls and `ThemeContext` bridges (implies `bridge-ui`) |
| `sub-app-extract` | `SubApp` extraction, `.wesl` 2D/3D materials, mesh-shader pipelines, and bounded `DeviceLost` recovery (`ModRenderRecoveryPlugin`; implies `bridge-asset`) |
| `bridge-reflect` | Dynamic `bevy_reflect`, `bevy_remote` BRP, `bevy-settings`, `ModSaveGamePlugin` (`HandleSerializeProcessor`/`HandleDeserializeProcessor` round-trip), and `ModTelemetryPlugin` |

## Verification

```bash
cargo test -p bamabi --no-default-features
cargo test -p bamabi --features signature-verification
cargo test -p bamabi --all-features
cargo clippy -p bamabi --all-targets --all-features -- -D warnings
cargo tree --edges normal -p bamabi --no-default-features --features preset-2d | sed 's/ (.*//' | grep -E 'bevy_pbr|bevy_light|bevy_solari|bevy_camera_controller'   # must print nothing
RUSTDOCFLAGS="-D warnings" cargo doc -p bamabi --no-deps --all-features
```

## Guest Mods, Developer CLI & Documentation

- **Rust Guest SDK (zero `bevy` dependencies):** `bamabi_guest` (in [`bamabi-wire`](https://github.com/yaga-simha/bamabi-wire))
- **Developer CLI (`new`, `build`, `pack`, `watch`, `export-sdk`, `check-mods`, `doctor`, `keygen`, `sign`, `verify`):** `cargo-bam` (in [`bamabi-tools`](https://github.com/yaga-simha/bamabi-tools))
- **Federated Documentation:** [`README.md`](../../README.md), [`bamabi-wire/spec.md`](https://github.com/yaga-simha/bamabi-wire/blob/main/spec.md), [`bamabi-core/docs/ARCHITECTURE.md`](https://github.com/yaga-simha/bamabi-core/blob/main/docs/ARCHITECTURE.md), and [`bamabi-showcase/docs/book/src/SUMMARY.md`](https://github.com/yaga-simha/bamabi-showcase/blob/main/docs/book/src/SUMMARY.md).

## License

Dual-licensed under `MIT OR Apache-2.0` ([`LICENSE-MIT`](../../LICENSE-MIT) and [`LICENSE-APACHE`](../../LICENSE-APACHE)).
