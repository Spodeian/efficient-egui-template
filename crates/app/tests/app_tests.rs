use app::TemplateApp;
use eframe::{App, Storage};
use shared::{AppState, Item, Priority};
use std::collections::HashMap;

#[derive(Default)]
struct MockStorage {
    data: HashMap<String, String>,
}

impl Storage for MockStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.data.insert(key.to_owned(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.data.remove(key);
    }
    fn flush(&mut self) {}
}

#[test]
fn test_template_app_initialization() {
    let app = TemplateApp::default();
    assert_eq!(app.state.collection.total_count(), 4);
    assert_eq!(app.state.collection.completed_count(), 1);
}

#[test]
fn test_template_app_save_and_load() {
    let mut storage = MockStorage::default();
    let mut app = TemplateApp::default();

    // Add item and toggle
    let new_item = Item::new(
        "custom-persistent-1",
        "Persistent Task",
        "Should be saved in storage",
        Priority::High,
    );
    app.state.collection.add(new_item);
    app.state.collection.toggle("custom-persistent-1");

    // Save
    app.save(&mut storage);

    // Verify storage contents
    let serialized = storage
        .get_string(eframe::APP_KEY)
        .expect("App key must exist in storage");
    assert!(serialized.contains("Persistent Task"));

    // Simulate reload from storage
    let loaded_state: AppState = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
    assert_eq!(loaded_state.collection.total_count(), 5);
    assert_eq!(loaded_state.collection.completed_count(), 2);
}

#[test]
fn test_template_app_export_dialog() {
    let mut app = TemplateApp::default();
    app.open_export_dialog(app::ExportFormat::Json);

    assert!(app.show_export_dialog.is_some());
    assert!(
        app.export_text_buffer
            .contains("Deploy Serverless WebApp")
    );
}

#[test]
fn test_template_app_load_multi_tier_json_and_ron() {
    use app::storage_manager::{
        DEDICATED_STORAGE_KEY, deserialize_app_state, load_state_multi_tier,
    };

    let mut original_state = AppState::default();
    original_state.collection.add(Item::new(
        "task-a",
        "Task A",
        "Description A",
        Priority::High,
    ));

    // 1. JSON roundtrip
    let json_str = serde_json::to_string(&original_state).unwrap();
    let from_json = deserialize_app_state(&json_str).expect("JSON should deserialize");
    assert_eq!(from_json.collection.total_count(), 5);

    // 2. RON roundtrip
    let ron_str = ron::to_string(&original_state).unwrap();
    let from_ron = deserialize_app_state(&ron_str).expect("RON should deserialize");
    assert_eq!(from_ron.collection.total_count(), 5);

    // 3. Load from dedicated key in storage (JSON)
    let mut storage_dedicated = MockStorage::default();
    storage_dedicated.set_string(DEDICATED_STORAGE_KEY, json_str.clone());
    let loaded_dedicated =
        load_state_multi_tier(Some(&storage_dedicated)).expect("Should load from dedicated key");
    assert_eq!(loaded_dedicated.collection.total_count(), 5);

    // 4. Load from legacy app key in storage (JSON)
    let mut storage_json_app = MockStorage::default();
    storage_json_app.set_string(eframe::APP_KEY, json_str);
    let loaded_json_app =
        load_state_multi_tier(Some(&storage_json_app)).expect("Should load from app key (JSON)");
    assert_eq!(loaded_json_app.collection.total_count(), 5);

    // 5. Load from legacy app key in storage (RON)
    let mut storage_ron_app = MockStorage::default();
    storage_ron_app.set_string(eframe::APP_KEY, ron_str);
    let loaded_ron_app =
        load_state_multi_tier(Some(&storage_ron_app)).expect("Should load from app key (RON)");
    assert_eq!(loaded_ron_app.collection.total_count(), 5);
}

#[test]
fn test_storage_diagnostics_defaults() {
    let diag = app::storage_manager::StorageDiagnostics::default();
    assert_eq!(diag.memory_items_count, 0);
    assert_eq!(diag.memory_capacity_limit, app::storage_manager::MEMORY_WORKING_SET_LIMIT);
    assert!(!diag.fallback_tier_active);
    assert!(!diag.quota_exceeded);
}
