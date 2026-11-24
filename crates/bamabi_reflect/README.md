# `bamabi_reflect`

Safe (`#![forbid(unsafe_code)]`) dynamic `bevy_reflect`, Bevy Remote Protocol (`bevy_remote` BRP), `bevy-settings` TOML, persistent save-game archives, and live telemetry bridges for [`bamabi`](../bamabi) (`ABI 0.4.0`, `v0.4.0`, Layer 3 in [`bamabi`](../../README.md)).

## Standalone Usage

Enable `bridge-reflect` on [`bamabi`](../bamabi) or depend directly on `bamabi_reflect` from GitHub:

```toml
[dependencies]
bamabi_reflect = { git = "https://github.com/yaga-simha/bamabi.git", branch = "main" }
```

## Overview

- **`ModReflectPlugin` / `ModReflectAppExt` / `ModReflectWorldExt`:** Synthesizes `DynamicStruct`, `ReflectFromPtr`, `ReflectComponent`, and `ReflectResource` registrations from both `ModComponent` witness schemas and declarative ABI `0.4` `BamFieldDesc` descriptors (including `FfiAssetHandle`, working seamlessly for raw C99/C++17/Zig/BQL types without Rust witness VTables), installs 6 `bamabi/*` JSON-RPC BRP methods alongside standard Bevy `world.*` BRP, and synchronizes mod configuration resources with persistent `bevy-settings` TOML files.
- **`ModSaveGamePlugin` / `ModSaveAppExt` / `ModSaveWorldExt`:** Persists `!ModScoped` modded entities into versioned JSON or checksummed `BAMSAVE1` binary `ModSaveArchive` snapshots (recording `mod_id`, `semver`, `component_uuid`, `schema_fingerprint`, layout, and fields) with Bevy `0.20.1` `HandleSerializeProcessor` / `HandleDeserializeProcessor` (and `EphemeralHandleBehavior`) round-tripping stable `mod://` asset handles cleanly, plus three recovery policies: `StrictRequireMods` (atomic mismatch rejection), `StripMissingModComponents` (graceful partial load with `ModSaveLoadReport`), and `MigrateOnLoad` (automatic field-name-matched `BamFieldDesc`/witness + custom `ModComponentMigrator` upgrades).
- **`ModTelemetryPlugin` / `ModTelemetryRemoteExt` / `ModTelemetryWorldExt`:** Installs 4 `bam/*` BRP JSON-RPC methods (`bam/list_mods`, `bam/system_metrics`, `bam/component_schemas` with exact `BamFieldDesc` offsets, and deferred `bam/hot_reload`), automatically ingests `ModBudgetTelemetry::system_budget_snapshots()`, and streams 64-byte little-endian `SystemTelemetryDigest64` frames (`p50_us`, `p99_us`, `summary_tick` table skips, fault/quarantine/budget flags) over non-blocking `AF_UNIX` Unix domain sockets or cross-platform loopback TCP.

## Verification

```bash
cargo test -p bamabi_reflect --all-features
cargo clippy -p bamabi_reflect --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p bamabi_reflect --no-deps --all-features
```

## License

Dual-licensed under `MIT OR Apache-2.0` ([`LICENSE-MIT`](../../LICENSE-MIT) and [`LICENSE-APACHE`](../../LICENSE-APACHE)).
