use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

const ORDERED_SETS_MOD_FALLBACK_C: &str = r#"#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include "bamabi.h"

static BamResult noop_init(
    const BamHostApi* api,
    BamLoadCtx* load_ctx,
    BamInstance** instance_out
) {
    (void)api;
    if (load_ctx == NULL || instance_out == NULL) {
        return BAM_RESULT_NULL_POINTER;
    }
    *instance_out = NULL;
    return BAM_RESULT_OK;
}

static void noop_shutdown(BamInstance* instance) {
    (void)instance;
}

static const BamManifest MANIFEST = {
    .struct_size = (uint32_t)sizeof(BamManifest),
    .abi_version = BAM_ABI_VERSION,
    .name = "ordered_sets_mod",
    .version = "0.2.0",
    .flags = 0u,
    .reserved0 = 0u,
    .save_state = NULL,
    .restore_state = NULL,
};

BAM_EXPORT const BamManifest* bamabi_manifest(void) { return &MANIFEST; }
BAM_EXPORT BamResult bamabi_init(
    const BamHostApi* api,
    BamLoadCtx* load_ctx,
    BamInstance** instance_out
) {
    return noop_init(api, load_ctx, instance_out);
}
BAM_EXPORT void bamabi_shutdown(BamInstance* instance) { noop_shutdown(instance); }
"#;

fn dylib_filename(stem: &str) -> String {
    let prefix = if cfg!(windows) { "" } else { "lib" };
    let ext = std::env::consts::DLL_EXTENSION;
    format!("{prefix}{stem}.{ext}")
}

fn compile_c99_cdylib(c_src: &Path, inc_dir: &Path, work_dir: &Path, out_dylib: &Path) {
    let candidates: Vec<std::ffi::OsString> = if let Some(cc) = std::env::var_os("CC") {
        vec![cc]
    } else if cfg!(windows) {
        ["clang", "clang-cl", "cl.exe", "cl", "gcc", "cc"]
            .into_iter()
            .map(Into::into)
            .collect()
    } else {
        ["cc", "clang", "gcc"].into_iter().map(Into::into).collect()
    };

    for cc in &candidates {
        let stem = Path::new(cc)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let is_msvc = stem.eq_ignore_ascii_case("cl") || stem.eq_ignore_ascii_case("clang-cl");
        let mut cmd = Command::new(cc);
        if is_msvc {
            cmd.args(["/nologo", "/LD", "/TC", "/std:c11", "/O2"])
                .arg(format!("/I{}", inc_dir.display()))
                .arg(format!("/Fe:{}", out_dylib.display()))
                .arg(format!("/Fo:{}/", work_dir.display()))
                .arg(c_src);
        } else {
            cmd.args(["-std=c99", "-O2", "-shared"]);
            if !cfg!(windows) {
                cmd.arg("-fPIC");
            }
            cmd.arg("-I")
                .arg(inc_dir)
                .arg("-o")
                .arg(out_dylib)
                .arg(c_src);
        }
        if let Ok(output) = cmd.output() {
            assert!(
                output.status.success(),
                "C compiler {:?} failed on {}: stdout={}, stderr={}",
                cc,
                c_src.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
    }
    panic!("no working C compiler found for {}", c_src.display());
}

pub(crate) fn fixture_dylib(stem: &str) -> PathBuf {
    assert_eq!(stem, "ordered_sets_mod", "unexpected render fixture stem");
    static BUILT_PATH: OnceLock<PathBuf> = OnceLock::new();
    BUILT_PATH
        .get_or_init(|| {
            let fname = dylib_filename("ordered_sets_mod");
            if let Some(dir) = std::env::var_os("BAM_FIXTURE_DIR").filter(|v| !v.is_empty()) {
                let candidate = PathBuf::from(dir).join(&fname);
                if candidate.is_file() {
                    return candidate;
                }
            }

            let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
            let ws_root = manifest_dir
                .parent()
                .and_then(Path::parent)
                .expect("bamabi workspace root");
            let out_dir = ws_root.join("target/fixtures");
            std::fs::create_dir_all(&out_dir).expect("create target/fixtures");
            let final_path = out_dir.join(&fname);
            if final_path.is_file() {
                return final_path;
            }

            let stage_dir =
                std::env::temp_dir().join(format!("bamabi_render_fixture_{}", std::process::id()));
            std::fs::create_dir_all(&stage_dir).expect("create render fixture stage dir");
            std::fs::write(
                stage_dir.join("bamabi.h"),
                bamabi_sdk_gen::render_c99_header().expect("render bamabi.h"),
            )
            .expect("write bamabi.h");

            let core_c = std::env::var_os("BAM_CORE_DIR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .or_else(|| ws_root.parent().map(|p| p.join("bamabi-core")))
                .map(|core_root| core_root.join("tests/fixtures/c_mods/ordered_sets_mod.c"));
            let c_src = if let Some(core_c) = core_c.filter(|p| p.is_file()) {
                core_c
            } else {
                let fallback = stage_dir.join("ordered_sets_mod.c");
                std::fs::write(&fallback, ORDERED_SETS_MOD_FALLBACK_C)
                    .expect("write fallback ordered_sets_mod.c");
                fallback
            };

            let staged_out = stage_dir.join(&fname);
            compile_c99_cdylib(&c_src, &stage_dir, &stage_dir, &staged_out);
            let tmp = out_dir.join(format!(".{fname}.{}.tmp", std::process::id()));
            std::fs::copy(&staged_out, &tmp).expect("stage fixture copy");
            match std::fs::rename(&tmp, &final_path) {
                Ok(()) => {}
                Err(_) if final_path.is_file() => {
                    let _ = std::fs::remove_file(&tmp);
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp);
                    panic!("failed to install {}: {e}", final_path.display());
                }
            }
            let _ = std::fs::remove_dir_all(&stage_dir);
            final_path
        })
        .clone()
}

pub(crate) struct TempDylibCopy {
    dir: PathBuf,
    path: PathBuf,
}

impl TempDylibCopy {
    pub(crate) fn new(original: &Path, tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let ext = original
            .extension()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dylib".to_owned());
        let dir = std::env::temp_dir().join(format!(
            "bamabi_render_{label_or_tag}_{}_{}",
            std::process::id(),
            id,
            label_or_tag = tag,
        ));
        std::fs::create_dir_all(&dir).expect("create shadow dir");
        let path = dir.join(format!("{tag}_{id}.{ext}"));
        std::fs::copy(original, &path).expect("copy shadow dylib");
        Self { dir, path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDylibCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
