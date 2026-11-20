# `bamabi_render`

Safe (`#![forbid(unsafe_code)]`) multi-world `SubApp` extraction, data-driven `.wesl` shaders, `AlignedVec`-backed `ShaderBuffer` storage assets, 2D/3D material modding, opt-in mesh-shader pipelines, and bounded GPU device-loss recovery for [`bamabi`](../bamabi) (`ABI 0.3.0`, `v0.3.0`, Layer 3 in [`bamabi`](../../README.md)).

## Standalone Usage

Enable `sub-app-extract` on [`bamabi`](../bamabi) or depend directly on `bamabi_render` from GitHub:

```toml
[dependencies]
bamabi_render = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main" }
```

## Overview

- **`ModSubAppPlugin<L>` / `ModSubAppExt` / `ModSubWorldExt`:** Extracts host-registered and guest-dynamic `BamComponent` components and singleton resources from `MainWorld` into arbitrary Bevy `SubApp`s (`RenderApp`, custom `AppLabel`s) via `bevy_extract`, aligned with Bevy `0.20`'s system-driven `RenderGraph` / `Core2d` / `Core3d` schedules.
- **`ModWeslMaterialPlugin` / `ModMaterialAppExt` / `ModMaterialWorldExt`:** Registers `ModId`-scoped `.wesl`/`.wgsl` `Shader` assets, aligned `ShaderBuffer`s, `SpriteMeshMaterial`, `ExtendedMaterial2d<SpriteMeshMaterial, ModSpriteExtension2d>`, and 3D `StandardMaterial` assets with automatic unload cleanup hooked into `bamabi_host::ModUnloadHooks`.
- **`ModSpriteMaterialPlugin` / `ModSpriteMeshAppExt` / `ModSpriteMeshWorldExt`:** Runtime `.wesl` 2D sprite material specialization (`ModWeslSpriteExtension2d`, `ModExtendedSpriteMaterial2d`, `ModSpriteMaterial`) and unified Bevy `0.20` `Mesh2d + SpriteMaterial` spawning, attachment, live shader/param hot-swap, `RenderStartup` cache invalidation, and unload cleanup.
- **`ModMeshShaderPlugin` / `ModMeshShaderSubAppPlugin<L>`:** Explicit opt-in mesh-shader pipeline bridge (`ModMeshShaderConfig`, per-mod pipeline quotas, `WgpuFeatures::EXPERIMENTAL_MESH_SHADER`, `ExtractedModMeshPipelines` extraction into `RenderApp` / custom `SubApp`s) with `RenderStartup` stale-pipeline clearing and deterministic eviction on `unload_mod`.
- **`ModRenderRecoveryPlugin` / `ModRenderRecoveryPolicy` / `ModRenderRecoveryTelemetry`:** Bounded Bevy `0.20` `RenderErrorHandler` + `RenderErrorPolicy::Recover` recovery for `ErrorType::DeviceLost` (configurable retry budgets & sliding recovery windows, circuit-breaker halt on non-`DeviceLost` errors or exhausted budgets, `RenderStartup` pipeline & 2D/3D material specialization cache reset, `ShaderBuffer` `MAIN_WORLD` CPU-copy preservation, and extensible `ModRenderRecoveryHooks`).

## Verification

```bash
cargo test -p bamabi_render --all-features
cargo clippy -p bamabi_render --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p bamabi_render --no-deps --all-features
```

## License

Dual-licensed under `MIT OR Apache-2.0` ([`LICENSE-MIT`](../../LICENSE-MIT) and [`LICENSE-APACHE`](../../LICENSE-APACHE)).
