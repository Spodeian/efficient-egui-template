//! Unified multi-tiered storage engine, persistence manager, PWA install bridge, and diagnostics.
//!
//! Core storage routing, diagnostics, versioned migration, and download triggers are delegated
//! to `spodeian_persist`, while retaining backwards-compatible RON fallback for legacy
//! desktop and `eframe::Storage` entries.

pub use spodeian_persist::{
    KeyValueStorage, MEMORY_WORKING_SET_LIMIT, StorageBackend, StorageDiagnostics, StorageError,
    query_storage_diagnostics, request_persistent_storage, trigger_binary_download,
    trigger_pwa_install, trigger_text_download,
};

use tracing::{info, warn};

/// Dedicated storage key used across `eframe::Storage`, `localStorage`, and native CAS routing.
pub const DEDICATED_STORAGE_KEY: &str = "serverless_template_app_state";

/// Read-only adapter exposing an `&dyn eframe::Storage` as [`spodeian_persist::KeyValueStorage`].
pub struct EframeStorageAdapter<'a>(pub &'a dyn eframe::Storage);

impl KeyValueStorage for EframeStorageAdapter<'_> {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get_string(key)
    }

    fn set_string(&mut self, _key: &str, _value: String) {}
}

fn finalize_loaded_state(mut state: shared::AppState) -> shared::AppState {
    if state.schema_version == 0 {
        state.schema_version = shared::CURRENT_SCHEMA_VERSION;
    }
    state
}

/// Dual-format deserializer for [`shared::AppState`], delegating versioned/raw JSON parsing to
/// `spodeian_persist::StateMigrator` and falling back to legacy RON payloads.
pub fn deserialize_app_state(content: &str) -> Result<shared::AppState, StorageError> {
    spodeian_persist::StateMigrator::deserialize_with_fallback(content, |trimmed| {
        ron::from_str::<shared::AppState>(trimmed).map_err(|e| e.to_string())
    })
    .map(finalize_loaded_state)
}

/// Multi-tiered loader for [`shared::AppState`].
/// Checks `eframe::Storage` (dedicated and legacy keys, JSON or RON) and browser `localStorage`
/// on `wasm32` via `spodeian_persist::load_state_multi_tier_keys` with RON fallback.
pub fn load_state_multi_tier(storage: Option<&dyn eframe::Storage>) -> Option<shared::AppState> {
    let adapter = storage.map(EframeStorageAdapter);
    if let Some(state) = spodeian_persist::load_state_multi_tier_keys::<shared::AppState>(
        adapter.as_ref().map(|a| a as &dyn KeyValueStorage),
        &[DEDICATED_STORAGE_KEY, eframe::APP_KEY],
        None,
    ) {
        return Some(finalize_loaded_state(state));
    }

    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window()
        && let Ok(Some(local_storage)) = window.local_storage()
    {
        for &key in &[DEDICATED_STORAGE_KEY, eframe::APP_KEY] {
            if let Ok(Some(content)) = local_storage.get_item(key) {
                match deserialize_app_state(&content) {
                    Ok(state) => {
                        info!("Restored legacy RON state from localStorage [{key}]");
                        return Some(state);
                    }
                    Err(e) => {
                        warn!("Failed to parse state from localStorage [{key}]: {e}");
                    }
                }
            }
        }
    }

    if let Some(storage) = storage {
        for &key in &[DEDICATED_STORAGE_KEY, eframe::APP_KEY] {
            if let Some(raw) = storage.get_string(key) {
                match deserialize_app_state(&raw) {
                    Ok(state) => {
                        info!("Restored legacy state from eframe::Storage [{key}]");
                        return Some(state);
                    }
                    Err(e) => {
                        warn!("Failed to parse state from eframe::Storage [{key}]: {e}");
                    }
                }
            }
        }

        if let Some(state) = eframe::get_value::<shared::AppState>(storage, eframe::APP_KEY) {
            info!("Restored AppState from eframe::get_value (RON).");
            return Some(finalize_loaded_state(state));
        }
    }

    None
}

/// What a save did, reported back to the UI once the write has finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The state is durable in this tier.
    Saved(StorageBackend),
    /// The write failed; the message says why.
    Failed(String),
}

/// Browser persistence: the whole state is one IndexedDB record, read once at startup and
/// written asynchronously. Saves run one at a time, and a burst of changes collapses into
/// one write of the latest state, so the UI thread never blocks on storage.
#[cfg(target_arch = "wasm32")]
mod web_store {
    use super::{DEDICATED_STORAGE_KEY, SaveOutcome, StorageBackend, deserialize_app_state};
    use spodeian_cache::web::IndexedDbStore;
    use std::cell::{Cell, RefCell};

    const DB_NAME: &str = "spodeian-egui-template";
    const STORE_NAME: &str = "state";

    thread_local! {
        static DB: RefCell<Option<IndexedDbStore>> = const { RefCell::new(None) };
        static CTX: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
        static PENDING: RefCell<Option<String>> = const { RefCell::new(None) };
        static SAVING: Cell<bool> = const { Cell::new(false) };
        static LAST_SAVED: RefCell<Option<String>> = const { RefCell::new(None) };
        static LOADED: RefCell<Option<Option<shared::AppState>>> = const { RefCell::new(None) };
        static OUTCOME: RefCell<Option<SaveOutcome>> = const { RefCell::new(None) };
    }

    fn repaint() {
        CTX.with(|c| {
            if let Some(ctx) = c.borrow().as_ref() {
                ctx.request_repaint();
            }
        });
    }

    fn report(outcome: SaveOutcome) {
        OUTCOME.with(|o| *o.borrow_mut() = Some(outcome));
        repaint();
    }

    pub fn open(ctx: &egui::Context) {
        CTX.with(|c| *c.borrow_mut() = Some(ctx.clone()));
        wasm_bindgen_futures::spawn_local(async {
            let db = match IndexedDbStore::open(DB_NAME, STORE_NAME).await {
                Ok(db) => db,
                Err(e) => {
                    return report(SaveOutcome::Failed(format!("Could not open storage: {e}")));
                }
            };
            let saved = match db.get_bytes(DEDICATED_STORAGE_KEY).await {
                Ok(bytes) => bytes.and_then(|b| String::from_utf8(b).ok()),
                Err(e) => {
                    report(SaveOutcome::Failed(format!(
                        "Could not load saved data: {e}"
                    )));
                    None
                }
            };
            let state = saved.as_deref().and_then(|json| {
                deserialize_app_state(json)
                    .map_err(|e| tracing::warn!("Ignoring unreadable saved state: {e}"))
                    .ok()
            });
            if state.is_some() {
                // Saves queued before the load would overwrite the record with the
                // startup state; the loaded state replaces that state instead.
                PENDING.with(|p| p.borrow_mut().take());
                LAST_SAVED.with(|l| *l.borrow_mut() = saved);
            }
            LOADED.with(|l| *l.borrow_mut() = Some(state));
            DB.with(|d| *d.borrow_mut() = Some(db));
            repaint();
            run_saves();
        });
    }

    pub fn take_loaded() -> Option<Option<shared::AppState>> {
        LOADED.with(|l| l.borrow_mut().take())
    }

    pub fn take_outcome() -> Option<SaveOutcome> {
        OUTCOME.with(|o| o.borrow_mut().take())
    }

    pub fn queue(json: String) {
        if LAST_SAVED.with(|l| l.borrow().as_deref() == Some(json.as_str())) {
            return; // unchanged since the last write
        }
        PENDING.with(|p| *p.borrow_mut() = Some(json));
        run_saves();
    }

    fn run_saves() {
        let Some(db) = DB.with(|d| d.borrow().clone()) else {
            return; // still opening; `open` starts the saves once the load is done
        };
        if PENDING.with(|p| p.borrow().is_none()) || SAVING.with(|s| s.replace(true)) {
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            while let Some(json) = PENDING.with(|p| p.borrow_mut().take()) {
                match db.put_bytes(DEDICATED_STORAGE_KEY, json.as_bytes()).await {
                    Ok(()) => {
                        let first_save = LAST_SAVED.with(|l| l.borrow().is_none());
                        LAST_SAVED.with(|l| *l.borrow_mut() = Some(json));
                        if first_save {
                            remove_legacy_copies();
                        }
                        report(SaveOutcome::Saved(StorageBackend::IndexedDb));
                    }
                    Err(e) => report(SaveOutcome::Failed(format!(
                        "Changes could not be saved: {e}"
                    ))),
                }
            }
            SAVING.with(|s| s.set(false));
        });
    }

    /// Earlier versions kept the state in local storage, mirrored into a write-only
    /// IndexedDB database. Once this store holds it, those copies are only stale data taking
    /// up quota (local storage has about 5 MB).
    fn remove_legacy_copies() {
        let Some(window) = web_sys::window() else {
            return;
        };
        if let Ok(Some(storage)) = window.local_storage() {
            for key in [DEDICATED_STORAGE_KEY, eframe::APP_KEY] {
                let _ = storage.remove_item(key);
            }
        }
        if let Ok(Some(factory)) = window.indexed_db() {
            let _ = factory.delete_database("app_template_offline_store");
        }
    }
}

/// Starts loading the durable state. In the browser this opens IndexedDB; the result
/// arrives through [`take_loaded_state`]. Native builds load synchronously instead.
pub fn open_durable_store(ctx: &egui::Context) {
    #[cfg(target_arch = "wasm32")]
    web_store::open(ctx);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = ctx;
}

/// The state loaded from IndexedDB, once: `Some(None)` means nothing was saved there yet.
/// Always `None` on native.
pub fn take_loaded_state() -> Option<Option<shared::AppState>> {
    #[cfg(target_arch = "wasm32")]
    return web_store::take_loaded();
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// The result of the most recent finished save, once.
pub fn take_save_outcome() -> Option<SaveOutcome> {
    #[cfg(target_arch = "wasm32")]
    return web_store::take_outcome();
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// Queues `state` to be written. In the browser the write is asynchronous and its result
/// arrives through [`take_save_outcome`]; an unchanged state is not rewritten. Native builds
/// persist through `eframe::App::save` (on exit and every 30 seconds), so this does nothing
/// there and reports the state as kept.
pub fn queue_state_save(state: &shared::AppState) -> Result<StorageBackend, StorageError> {
    #[cfg(target_arch = "wasm32")]
    {
        let json = serde_json::to_string(state)
            .map_err(|e| StorageError::DeserializationFailed(e.to_string()))?;
        web_store::queue(json);
        Ok(StorageBackend::IndexedDb)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = state;
        Ok(StorageBackend::DiskFile)
    }
}
