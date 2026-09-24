# Serverless & Desktop egui Template Documentation Standards

**egui & eframe Architecture, Multi-Tier Persistence Conventions, and Quality Enforcement**

---

## 1. Overview & Core Philosophy

This template provides a modular architecture for cross-platform desktop and serverless web applications. All public APIs, UI components, state persistence mechanisms, and domain models must be clearly documented with zero broken intra-doc links.

### The Five Pillars:
1. **Architectural Clarity**: Clear separation between domain models (`crates/shared`), view-controller (`crates/app`), desktop runner (`crates/desktop`), and WASM entrypoint (`crates/web`).
2. **Persistence Rigor**: Explicit documentation of multi-tier storage keys, serialization schemes (JSON/BSON/RON), quota limits, and migration pathways.
3. **Responsive UI Guidelines**: Documentation of screen constraint adaptations (compact mobile portrait vs. widescreen desktop).
4. **Zero-Warning Hygiene**: `cargo doc --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` must compile with zero warnings.
5. **Strict Test Isolation**: All tests reside in dedicated test files under `tests/`; no inline tests inside production source files.

---

## 2. Doc Comments & intra-doc links

- Document all public structs, enums, fields, and methods using triple slashes `///`.
- Use markdown backticks for code identifiers: `[`ThemeMode`]`, `[`AppState`]`.
- Escape square brackets in formulas or arrays: `\[0, 1\]`.

---

## 3. Verification Checklist

Before opening PRs to `main`:
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes with 0 warnings.
- [ ] `cargo test --workspace` passes 100% of integration tests.
- [ ] No `DOCUMENTATION_STANDARDS.md` or internal roadmap files are included on `main`.
