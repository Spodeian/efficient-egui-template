# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Screen reader support on the web: `crates/web` registers `egui-web-accesskit` (tag `v0.36.4`, matching egui 0.36), which mirrors egui's AccessKit tree into an HTML/ARIA overlay on the canvas so screen readers can read and operate the widgets. It is part of the `accesskit` feature; desktop keeps eframe's built-in AccessKit adapters.

### Fixed
- spodeian-core v3.3.0: native content stores now delete superseded data instead of keeping every version on disk.
- Browser storage: the state was written to IndexedDB on every change but never read back, so a reload restored whatever local storage held. Local storage only got the state when it was under 16 KB, or every 30 seconds through eframe. IndexedDB (`spodeian-cache` `IndexedDbStore`) is now the durable copy. It is read once at startup and written asynchronously, one save at a time, with bursts coalesced and unchanged states skipped. "✓ Saved" shows only after the browser confirms the write. Copies left by earlier versions in local storage and the old IndexedDB database are removed after the first save.
- Each change no longer writes the whole state two or three times: to local storage, the JS IndexedDB bridge and (on native) a new temp-dir blob that was never read or cleaned up. Native keeps eframe's storage file as its only copy; the unused JS IndexedDB bridge in `index.js` is gone.
- Service worker: content-hashed bundles are served cache-first with no background re-download on every visit. Other files are network-first with an offline fallback, and cache writes are kept alive with `waitUntil`.
- `_headers`: `/sw.js` also matched the `/*.js` immutable rule, so it was sent with conflicting `Cache-Control` values. It now detaches that header. HTML and `sw.js` use `no-cache` (a cheap 304 revalidation) instead of `no-store`.
- Generated projects no longer inherit the template's scaffold-check CI job, whose name check would fail there; it moved to `template.yml`, which cargo-generate leaves out.
- `cargo generate` failed with "Substitution skipped, found invalid syntax" and created no files: workflows, `crates/shared/src/export.rs` and binary assets are now copied without Liquid substitution.
- `CHANGELOG.md`, `CONTRIBUTING.md`, `Cargo.lock` and build output are no longer copied into generated projects (moved from `exclude` to `ignore`).
- Generated projects take their own name in `wrangler.toml` and the `repository` URL instead of deploying over this template's Worker; this repo's own deploy passes `--name` from the repository name.
- Projects generated with PWA or Cloudflare off no longer fail `trunk build` on the missing `sw.js`, `manifest.json` or `_headers` (`crates/web/index.html`).

### Changed
- `template-scaffold-check` CI now runs real `cargo generate` (defaults and every option off) instead of deleting crates to simulate it.
- Lint policy lives in `[workspace.lints]` (`unsafe_code = "deny"`, `clippy::unwrap_used = "warn"`) with `clippy.toml` allowing unwraps in tests; `missing_docs` stays on each library crate root.
- `escape_html` and the BSON import reuse `spodeian-export` instead of local copies.
- `base64` pinned to 0.22 to match spodeian-core, so builds carry one copy; Dependabot skips its minor/major bumps.

### Removed
- Unused dependencies: direct `miniz_oxide`, and core crates no crate called.

## [0.2.2] - 2026-11-18

### Added
- `spodeian-parallel` integration in `crates/shared` powering Rayon work-stealing sweeps on native desktops and seamless SIMD128 sequential execution on `wasm32` (`sample_points`, `batch_harmonic_sweep`).
- Bundled WebGPU WGSL compute kernel (`PARAMETRIC_HARMONIC_WGSL_SHADER`), CPU parity helper (`evaluate_harmonic_wgsl_cpu_reference`), `ComputeRuntimeSummary`, and `app::active_renderer_label()` (`wgpu` vs. `glow`) surfaced in Analytics & About views.
- Dedicated [`ARCHITECTURE.md`](ARCHITECTURE.md) and 3-template comparison matrix in [`README.md`](README.md).

## [0.2.1] - 2026-11-18

### Added
- `AppPage::Analytics` view with KPI `PerformanceMetricCard`s, a colorblind-safe priority distribution chart, and an interactive 2D pan/zoom parametric signal explorer (`ParametricPreviewConfig`).
- Self-contained SVG (`export_to_svg`), printable HTML (`export_to_printable_html`), and compressed `#state=<base64url>` shareable URL hash (`encode_shareable_url_hash` / `decode_shareable_url_hash`) interchange, plus a non-destructive shared-link preview banner (`is_viewing_shared_link`, restore/keep actions) and drag-and-drop file import (`.json`, `.csv`, `.bson`).
- `AccessibilityPreferences` (`ColorblindMode`, `reduced_motion`, `large_touch_targets`), embedded `ThemePickerWidget`, `Zen Mode` (`F11`), `ReactiveFramePacer`, and `LogConsoleWidget` inside the Telemetry modal.
- Runnable `crates/shared/examples/state_interchange_and_report.rs` example and expanded unit/`proptest` coverage for Analytics routing, SVG/HTML escaping, share-link round-trips, dropped-file imports, and Zen Mode.

### Changed
- Collapsed secondary navbar controls into an overflow `Tools` menu with inline status toasts, lowering horizontal clutter on narrow/mobile viewports, and lowered native desktop minimum window size to `320x240`.
- Hardened offline-first web builds by removing external CDN font/CSS `@import`s from `crates/web/index.scss`, adding safe-area insets and `100dvh`, enforcing strict `isHostOrSubdomain` checks in `crates/web/sw.js`, and stamping build IDs / precache lists in `deploy.sh`.
- Switched workspace `members` to `["crates/*"]` for clean `cargo-generate` scaffolding with `include_desktop = false`, enabled `#![warn(missing_docs)]` and `#![warn(clippy::unwrap_used)]` on `shared` and `app`, and expanded CI (`wasm32` clippy, `RUSTDOCFLAGS="-D warnings"`, examples/benches, >2 MB tracked-file check, and `template-scaffold-check`).

## [0.2.0] - 2026-11-14

### Added
- `AppState::schema_version` (`CURRENT_SCHEMA_VERSION = 1`) backed by `spodeian_persist::StateMigrator` for forward-compatible JSON snapshots and legacy unversioned / RON state upgrades.
- Searchable `spodeian_ui::CommandPaletteWidget` (`Ctrl+K` / `Cmd+K`), keyboard shortcut conflict detector (`detect_shortcut_conflicts`), and bounded 64-step `spodeian_ui::UndoStack` (`Ctrl+Z` / `Ctrl+Y`) across item edits, resets, and imports.
- Accessible `dyslexia_friendly` reading mode (`Aa` navbar toggle + `Ctrl+Shift+D`) and `ui_scale_percent` (`80%`–`150%` zoom factor) persisted in `AppConfig`.
- Recoverable panic/fault capture helper (`TemplateApp::run_recoverable`) and non-crashing recovery overlay banner (`render_error_recovery_banner`).
- Interactive `cargo-generate.toml` configuration for optional desktop runner, offline PWA, Cloudflare Pages, and mobile build scripts, plus `.github/dependabot.yml`.
- Property-based tests (`proptest`) covering versioned JSON/BSON round-trips (including `u64 >= 2^63` via `spodeian_export::u64_i64_compat`), RFC 4180 CSV round-trips, UI scale/theme normalization, `UndoStack` state invariants, `CommandPaletteWidget` search indexing, and shortcut conflict detection.

### Changed
- Deduplicated storage routing/diagnostics (`crates/app/src/storage_manager.rs`), telemetry dashboard DTOs and modal (`crates/shared/src/telemetry.rs`, `crates/app/src/components/telemetry_viewer.rs`), and CSV/JSON/BSON interchange (`crates/shared/src/export.rs`) onto `spodeian-core` (`spodeian-persist`, `spodeian-ui`, `spodeian-telemetry`, `spodeian-export`).
- Replaced the hardcoded `"Version 0.1.0"` label in the About view with dynamic `env!("CARGO_PKG_VERSION")`.

## [0.1.0] - 2026-08-18

### Added
- Initial 4-crate `egui 0.36` + `eframe 0.36` serverless WASM & native desktop template with AccessKit, multi-tier persistence, offline PWA service worker, JSON/CSV/BSON export, and Criterion serialization benchmarks.
