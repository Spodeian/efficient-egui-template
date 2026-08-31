# Serverless & Desktop Template

A production-ready, modular Rust & `egui` template designed for high-performance **Serverless Web (WASM / Cloudflare Pages / PWA)** and **Native Desktop (Windows, macOS, Linux)** applications.

Based directly on the proven architecture, production fixes, and deployment pipelines of [Revisited IPIP-NEO](https://github.com/Spodeian/Revisited-IPIP-NEO).

---

## 🏛️ Architectural Structure

The workspace is organized into four decoupled crates:

- **[`crates/shared`](crates/shared)**: Core domain models, business logic, configuration (`ThemeMode`, `AppConfig`, `AppState`), and interchange serialization (JSON and CSV import/export).
- **[`crates/app`](crates/app)**: UI view controller and layout layer powered by `egui` & `eframe`. Contains responsive design helpers (`ScreenConstraints`), the light/dark theme engine, modal dialogs, and persistent state management via `eframe::Storage`.
- **[`crates/desktop`](crates/desktop)**: Native runner configured with `tracing` logging and native window viewport settings.
- **[`crates/web`](crates/web)**: WebAssembly client entrypoint with `wasm-bindgen`, WebRunner initialization, and PWA / serverless assets (`index.html`, `index.css`, `sw.js`, `manifest.json`, `_headers`, `_redirects`).

---

## ✨ Key Features

- **⚡ Native & Serverless Dual-Target**: Compile identical application logic to desktop native executables or static client-side WASM bundles.
- **💾 Automatic Cross-Platform State Persistence**: Synchronizes local state seamlessly across browser refreshes and desktop restarts using `eframe::Storage`.
- **📱 Responsive & Touch-Friendly UI**: Layout automatically adapts across desktop displays, mobile viewports, and constrained height windows.
- **🎨 Theme Engine**: Built-in Dark Mode and a soothing, high-contrast Warm Light Mode.
- **⌨️ Intuitive Keyboard Support**: Press `Enter` to quickly submit new items and `Escape` to close open modal dialogs.
- **📦 Data Interchange & File Downloads**: Built-in JSON & CSV export/import dialogs with copy-to-clipboard toasts and direct browser/filesystem download triggers (`trigger_file_download`).
- **📶 Immutable Serverless PWA Caching**: Hybrid service worker caching strategy (**Network-First** for `index.html` to ensure atomic releases; **Cache-First** for immutable, content-hashed `.wasm`, `.js`, and `.css` assets) with offline fallback.
- **🛡️ SRI Minification Immunity**: Configured with `data-integrity="none"` to allow aggressive post-build asset minification (HTML, CSS, JS) without SRI hash mismatches or white screens.
- **🚀 Cloudflare Pages Build System v3 & GitHub Pages CI/CD**: Fully compliant with Cloudflare's modern v3 build image, automated toolchain installation (`rust-toolchain.toml`), SPA routing (`_redirects`), Cloudflare headers (`_headers`), Wrangler configuration (`wrangler.toml`), and GitHub Actions workflow (`.github/workflows/static.yml`).

---

## 🛠️ Build Requirements

- **Rust**: Automatically managed via [`rust-toolchain.toml`](rust-toolchain.toml) (installs stable with `wasm32-unknown-unknown`).
- **Trunk Bundler**:
  ```bash
  cargo install trunk
  ```
- *(Optional)* **wasm-opt** (Binaryen v132) for release size optimization.

---

## 🚀 Development Quickstart

### 1. Run Native Desktop App
```bash
cargo run -p desktop
```

### 2. Run Web App Locally
```bash
trunk serve
```
Open [http://localhost:8080](http://localhost:8080) in your browser. Live reloading is automatically enabled.

### 3. Run Test Suite
```bash
cargo test --workspace
```

### 4. Run Clippy Static Analysis
```bash
cargo clippy --workspace --all-targets
```

---

## 📦 Production Builds & Deployment

### Native Desktop Binary
```bash
cargo build -p desktop --release
```
The optimized executable will be located in `target/release/`.

### Cloudflare Pages (Build System v3)

Deploy directly to Cloudflare Pages using the automated build script:

```bash
bash deploy.sh
```

#### Cloudflare Pages Dashboard Settings:
- **Build System Version**: `v3` (2024/2026 build image)
- **Framework Preset**: `None` (or `Custom`)
- **Build Command**: `bash deploy.sh`
- **Build Output Directory**: `crates/web/dist`
- **Environment Variables**:
  - `RUST_VERSION`: `stable` (Optional if `rust-toolchain.toml` is present)
  - `CARGO_HOME`: `/opt/buildhome/.cargo`

#### Local Testing with Wrangler:
```bash
trunk build --release
wrangler pages dev
```

### GitHub Pages Deployment

The included workflow in [`.github/workflows/static.yml`](.github/workflows/static.yml) automatically builds and deploys your WASM web application to GitHub Pages whenever you push to `main`.

---

## 🧩 Customizing for Your App

1. **Rename Workspace & Metadata**: Update `name`, `version`, `authors`, and `description` in [`Cargo.toml`](Cargo.toml) and subcrate manifests.
2. **Define Domain Data**: Replace `Item` and `ItemCollection` in [`crates/shared/src/models.rs`](crates/shared/src/models.rs) with your application's data models.
3. **Build Views & Components**: Update [`crates/app/src/lib.rs`](crates/app/src/lib.rs) with your UI widgets, layouts, and panels.
4. **Update PWA & SEO Tags**: Customize `title`, meta description, OpenGraph tags, and icons in [`crates/web/index.html`](crates/web/index.html) and [`crates/web/manifest.json`](crates/web/manifest.json).

---

## 📄 License

This template is dual-licensed under [MIT](LICENSE) or Apache 2.0 at your option.
