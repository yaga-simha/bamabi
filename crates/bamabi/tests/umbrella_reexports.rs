#![forbid(unsafe_code)]

//! Verifies that the canonical top-level `bamabi` facade crate exports all core host APIs,
//! module paths (`bridges::*`, `recovery`, `sprite_mesh_shaders`, `sub_app_materials`, `savegame`,
//! `telemetry`, `watcher`), `prelude::*`, procedural derives (`#[derive(ModComponent, AbiPod)]`),
//! curated bridge presets (`preset-2d`, `preset-3d`, `preset-ui`, `preset-full`), and every
//! optional feature flag under both default features and `--all-features`.

use bamabi::prelude::*;
use bevy_app::App;
use bevy_ecs::component::Component;

#[repr(C)]
#[derive(Component, ModComponent, Clone, Copy, Default, Debug, PartialEq)]
#[mod_component(name = "umbrella_test::Health")]
struct UmbrellaHealth {
    current: f32,
    max: f32,
}

#[repr(C)]
#[derive(AbiPod, Clone, Copy, Default, Debug, PartialEq)]
struct UmbrellaPod {
    x: f32,
    y: f32,
}

#[test]
fn umbrella_core_exports_and_derives_work() {
    assert_eq!(bamabi::bamabi_sys::ABI_VERSION, 0x0003_0000);
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.3.0");

    let mut app = App::new();
    app.add_plugins(AbiModPlugin::with_error_policy(ModErrorPolicy::Quarantine));
    app.register_mod_component::<UmbrellaHealth>(
        ComponentOptions::new().with_raw_insert().with_default(),
    )
    .expect("register UmbrellaHealth through canonical facade crate");

    assert!(app.world().contains_resource::<ModHost>());
    assert!(app.world().contains_resource::<ModRegistry>());
    assert!(app.world().contains_resource::<ModUnloadHooks>());
    assert!(app.world().contains_resource::<ModBudgetPolicy>());
    assert!(app.world().contains_resource::<ModBudgetTelemetry>());

    let _ = core::any::type_name::<bamabi::budgets::ModBudgetLimits>();
    let _ = core::any::type_name::<bamabi::bamabi_host::ModHost>();
    let pod = UmbrellaPod { x: 1.0, y: 2.0 };
    assert_eq!(pod.x, 1.0);
    assert_eq!(pod.y, 2.0);
}

#[cfg(feature = "strict-ffi-checks")]
#[test]
fn umbrella_strict_ffi_checks_feature_forwarded() {
    let mut app = App::new();
    app.add_plugins(AbiModPlugin);
    assert!(app.world().contains_resource::<ModHost>());
}

#[cfg(feature = "diagnostics")]
#[test]
fn umbrella_diagnostics_reexports_work() {
    let path = bamabi::diagnostics::mod_cpu_time_path("umbrella_mod");
    assert!(path.as_str().contains("umbrella_mod"));
    let _ = &bamabi::diagnostics::TOTAL_CPU_TIME_US_PATH;
}

#[cfg(feature = "signature-verification")]
#[test]
fn umbrella_signature_verification_reexports_work() {
    let _ = core::any::type_name::<bamabi::ModTrustPolicy>();
    let _ = core::any::type_name::<bamabi::VerifiedModSignature>();
    let _ = core::any::type_name::<bamabi::BamPackBuilder>();
}

#[cfg(feature = "bridge-std-types")]
#[test]
fn umbrella_std_types_reexports_work() {
    let mut app = App::new();
    app.add_plugins((AbiModPlugin, bamabi::bridges::StdTypesBridgePlugin));
    let _t = bamabi::FfiTransform::IDENTITY;
    let _gt = bamabi::bridges::std_types::FfiGlobalTransform::IDENTITY;
    assert!(!bamabi::CANONICAL_DISABLED_NAME.is_empty());
}

#[cfg(feature = "bridge-asset")]
#[test]
fn umbrella_asset_reexports_work() {
    let _plugin = bamabi::ModAssetPlugin;
    let _reg = bamabi::bridges::asset::ModAssetRegistry::new();
    let _ = core::any::type_name::<bamabi::AssetOverlayPolicy>();
}

#[cfg(feature = "bridge-state")]
#[test]
fn umbrella_state_reexports_work() {
    let _ = core::any::type_name::<bamabi::ModStateError>();
    let _ = core::any::type_name::<bamabi::ModStateSlab>();
    let _ = core::any::type_name::<bamabi::bridges::state::ModStateError>();
}

#[cfg(all(
    feature = "bridge-std-types",
    feature = "bridge-asset",
    feature = "bridge-state"
))]
#[test]
fn umbrella_combined_bridges_plugin_reexport_works() {
    let _ = core::any::type_name::<bamabi::ModBridgesPlugin>();
    let _ = core::any::type_name::<bamabi::bridges::ModBridgesPlugin>();
}

#[cfg(feature = "bridge-bsn")]
#[test]
fn umbrella_bsn_reexports_work() {
    assert_eq!(
        bamabi::CANONICAL_READY_NAME,
        bamabi::bridges::bsn::CANONICAL_READY_NAME
    );
    let _reg = bamabi::HostSceneRegistry::new();
}

#[cfg(feature = "bridge-reflect")]
#[test]
fn umbrella_reflect_savegame_telemetry_reexports_work() {
    assert_eq!(
        bamabi::BRP_METHOD_LIST_COMPONENTS,
        bamabi::bridges::reflect::BRP_METHOD_LIST_COMPONENTS
    );
    let mut app = App::new();
    app.add_plugins((AbiModPlugin, ModReflectPlugin));
    assert!(app.world().contains_resource::<ModReflectRegistry>());
    let _ = core::any::type_name::<bamabi::ModSaveGamePlugin>();
    let _ = core::any::type_name::<bamabi::savegame::ModSaveArchive>();
    let _ = core::any::type_name::<bamabi::EphemeralHandleBehavior>();
    let _ = core::any::type_name::<bamabi::bridges::EphemeralHandleBehavior>();
    let _ = core::any::type_name::<bamabi::savegame::EphemeralHandleBehavior>();
    let _behavior: EphemeralHandleBehavior = EphemeralHandleBehavior::Warn;
    let _ = core::any::type_name::<bamabi::ModTelemetryPlugin>();
    let _ = core::any::type_name::<bamabi::telemetry::SystemTelemetryDigest64>();
    let _ = core::any::type_name::<bamabi::TelemetrySliceError>();
}

#[cfg(feature = "sub-app-extract")]
#[test]
fn umbrella_render_and_recovery_reexports_work() {
    let mut app = App::new();
    app.add_plugins((
        AbiModPlugin,
        ModWeslMaterialPlugin,
        ModRenderRecoveryPlugin::default(),
    ));
    assert!(app.world().contains_resource::<ModWeslMaterialRegistry>());
    assert!(app.world().contains_resource::<ModRenderRecoveryPolicy>());
    assert!(
        app.world()
            .contains_resource::<ModRenderRecoveryTelemetry>()
    );
    assert!(app.world().contains_resource::<ModRenderRecoveryHooks>());
    assert!(app.mod_render_recovery_telemetry().is_some());
    assert!(app.world().mod_render_recovery_telemetry().is_some());
    app.register_mod_render_recovery_hook("facade_smoke", |_main_world, _render_world| {});
    assert!(app.unregister_mod_render_recovery_hook("facade_smoke"));

    let _spec = bamabi::sub_app_materials::ModSpriteMaterialSpec::new("umbrella_smoke");
    let _params = ModMaterialParams::default();
    let _ext = ModSpriteExtension2d::default();
    let _ = core::any::type_name::<bamabi::ModSpriteMaterialPlugin>();
    let _ = core::any::type_name::<bamabi::ModMeshShaderPlugin>();
    let _ = core::any::type_name::<bamabi::sprite_mesh_shaders::ModUnifiedSpriteSpec>();
    let _ = core::any::type_name::<bamabi::ModRenderRecoveryPlugin>();
    let _ = core::any::type_name::<bamabi::ModRenderRecoveryPolicy>();
    let _ = core::any::type_name::<bamabi::ModRenderRecoveryTelemetry>();
    let _ = core::any::type_name::<bamabi::ModRenderRecoveryOutcome>();
    let _ = core::any::type_name::<bamabi::ModRenderRecoveryHooks>();
    let _ = core::any::type_name::<bamabi::recovery::ModRenderCacheInvalidationReport>();
    assert_eq!(app.cleanup_mod_materials(ModId(999_999)), 0);
}

#[cfg(feature = "hot-reload-watcher")]
#[test]
fn umbrella_watcher_reexports_work() {
    let _history = bamabi::ModHotReloadHistory::default();
    let _action = bamabi::watcher::ModHotReloadAction::Loaded;
}

#[cfg(feature = "bridge-picking")]
#[test]
fn umbrella_picking_and_input_reexports_work() {
    let _ = core::any::type_name::<bamabi::ModPickingBridgePlugin>();
    let _ = core::any::type_name::<bamabi::bridges::picking::FfiPointerClick>();
    let _ = core::any::type_name::<bamabi::bridges::input_ext::FfiGamepadState>();
}

#[cfg(feature = "bridge-audio")]
#[test]
fn umbrella_audio_reexports_work() {
    let _ = core::any::type_name::<bamabi::ModAudioBridgePlugin>();
    let _ = core::any::type_name::<bamabi::bridges::audio::FfiAudioPlayerSpec>();
    let _ = core::any::type_name::<bamabi::FfiAudioPlaybackSettings>();
}

#[cfg(feature = "bridge-camera-2d")]
#[test]
fn umbrella_camera_2d_reexports_work() {
    let _ = core::any::type_name::<bamabi::FfiCamera2d>();
    let _ = core::any::type_name::<bamabi::bridges::FfiCamera2d>();
    let _ = core::any::type_name::<bamabi::bridges::camera_light::FfiCamera2d>();
    let _ = core::any::type_name::<bamabi::ModCameraLightBridgePlugin>();
    let _ = core::any::type_name::<bamabi::CameraLightBridgeSystems>();
    let mut app = App::new();
    app.add_plugins((AbiModPlugin, ModCameraLightBridgePlugin));
    app.update();
}

#[cfg(feature = "bridge-light")]
#[test]
fn umbrella_camera_light_reexports_work() {
    let _ = core::any::type_name::<bamabi::ModCameraLightBridgePlugin>();
    let _ = core::any::type_name::<bamabi::FfiCamera2d>();
    let _ = core::any::type_name::<bamabi::bridges::camera_light::FfiCamera3d>();
    let _ = core::any::type_name::<bamabi::FfiPointLight>();
}

#[cfg(feature = "bridge-ui")]
#[test]
fn umbrella_ui_reexports_work() {
    let _ = core::any::type_name::<bamabi::ModUiBridgePlugin>();
    let _ = core::any::type_name::<bamabi::bridges::ui::FfiUiNode>();
    let _ = core::any::type_name::<bamabi::FfiTabList>();
}

#[cfg(feature = "bridge-feathers")]
#[test]
fn umbrella_feathers_reexports_work() {
    let _ = core::any::type_name::<bamabi::FfiThemeContext>();
    let _ = core::any::type_name::<bamabi::FfiFeathersNumberInput>();
}

#[cfg(feature = "bridge-camera-controller")]
#[test]
fn umbrella_camera_controller_reexports_work() {
    let _ = core::any::type_name::<bamabi::FfiPanOrbitCamera>();
}

#[cfg(feature = "bridge-solari")]
#[test]
fn umbrella_solari_reexports_work() {
    let _ = core::any::type_name::<bamabi::FfiSolariLighting>();
}

#[cfg(feature = "preset-2d")]
#[test]
fn umbrella_preset_2d_plugin_works() {
    let _ = core::any::type_name::<bamabi::ModPreset2dPlugin>();
    let _ = core::any::type_name::<bamabi::bridges::ModPreset2dPlugin>();
    let mut app = App::new();
    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        AbiModPlugin,
        ModPreset2dPlugin,
        bevy_asset::AssetPlugin::default(),
    ));
    app.update();
}

#[cfg(feature = "preset-3d")]
#[test]
fn umbrella_preset_3d_plugin_works() {
    let _ = core::any::type_name::<bamabi::ModPreset3dPlugin>();
    let _ = core::any::type_name::<bamabi::bridges::ModPreset3dPlugin>();
    let mut app = App::new();
    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        AbiModPlugin,
        ModPreset3dPlugin,
        bevy_asset::AssetPlugin::default(),
    ));
    app.update();
}

#[cfg(feature = "preset-ui")]
#[test]
fn umbrella_preset_ui_plugin_works() {
    let _ = core::any::type_name::<bamabi::ModPresetUiPlugin>();
    let _ = core::any::type_name::<bamabi::bridges::ModPresetUiPlugin>();
    let mut app = App::new();
    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        AbiModPlugin,
        ModPresetUiPlugin,
        bevy_asset::AssetPlugin::default(),
    ));
    app.update();
}

#[cfg(feature = "preset-full")]
#[test]
fn umbrella_preset_full_plugin_works() {
    let _ = core::any::type_name::<bamabi::ModPresetFullPlugin>();
    let _ = core::any::type_name::<bamabi::bridges::ModPresetFullPlugin>();
    let mut app = App::new();
    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        AbiModPlugin,
        ModPresetFullPlugin,
        bevy_asset::AssetPlugin::default(),
    ));
    app.update();
}
