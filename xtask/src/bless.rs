//! `bless`: the one command that regenerates every derived artefact from its source (operator
//! decision 11, lean-gates WP-23).
//!
//! Source to derived, one direction only: nothing here reads an artefact to decide what the source
//! should say. The legs, in order:
//!
//! 1. the kernel pin: `src_lines` and the `[kernel_src]` block of `xtask/kernel-core-baseline.toml`,
//!    from `crates/maos-kernel-core/src`;
//! 2. `STABILITY.md`, from workspace state;
//! 3. `docs/errors/error-catalog.json`, from `xtask/error-catalog.toml`;
//! 4. the ABI reference pages under `docs-site/abi/v1`, from the `maos-spirit-abi` rustdoc JSON;
//! 5. the example Spirits under `examples/`, from `templates/`.
//!
//! Every `--check` gate over one of these reds on a stale artefact and names this command. The
//! hand-written inputs (`error-catalog.toml`, the templates, the kernel source, the `STABILITY.md`
//! preserved fences) are never rewritten.
use std::path::Path;

pub fn run(cwd: &Path) -> Result<(), String> {
    let root = xtask::check_error_catalog::workspace_root_from_cwd(cwd)?;
    std::env::set_current_dir(&root)
        .map_err(|e| format!("cannot enter workspace root {}: {e}", root.display()))?;

    let pin_changed = crate::check_kernel_baseline::bless_pin(&root)?;
    eprintln!(
        "bless: xtask/kernel-core-baseline.toml: {}",
        if pin_changed {
            "updated"
        } else {
            "already in sync"
        }
    );

    crate::stability_matrix::run(&root, false, false)?;

    let catalog = root.join("xtask/error-catalog.toml");
    let artefact = root.join("docs/errors/error-catalog.json");
    xtask::check_error_catalog::run_generate(
        &catalog.display().to_string(),
        &artefact.display().to_string(),
    )?;

    crate::gen_abi_docs::run(None, false)?;
    eprintln!("bless: docs-site/abi/v1: regenerated");

    crate::templates_regen::run(&root, None, false, false)?;
    eprintln!("bless: examples/: regenerated from templates/");

    eprintln!(
        "bless: done; review `git diff` and commit the regenerated artefacts with their source"
    );
    Ok(())
}
