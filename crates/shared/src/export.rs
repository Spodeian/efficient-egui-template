//! Serialization and data interchange engines for JSON, CSV, and compressed BSON.

use crate::models::{AppConfig, AppState, Item, ItemCollection, Priority};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExportError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CSV parse error: {0}")]
    Csv(String),
    #[error("Invalid priority format: {0}")]
    InvalidPriority(String),
    #[error("Empty data provided")]
    EmptyData,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImportedJsonData {
    State(AppState),
    Collection(ItemCollection),
}

/// Serializes application state to formatted JSON.
pub fn export_to_json(state: &AppState) -> Result<String, ExportError> {
    serde_json::to_string_pretty(state).map_err(ExportError::Json)
}

/// Deserializes application state from a JSON string.
/// Transparently handles both full AppState payloads and standalone ItemCollection payloads.
pub fn import_from_json(json_str: &str) -> Result<AppState, ExportError> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Err(ExportError::EmptyData);
    }
    match serde_json::from_str::<ImportedJsonData>(trimmed) {
        Ok(ImportedJsonData::State(state)) => Ok(state),
        Ok(ImportedJsonData::Collection(col)) => Ok(AppState {
            collection: col,
            config: AppConfig::default(),
        }),
        Err(e) => Err(ExportError::Json(e)),
    }
}

/// Serializes an item collection to RFC 4180-compliant CSV.
pub fn export_to_csv(collection: &ItemCollection) -> Result<String, ExportError> {
    let mut csv = String::from("id,title,description,priority,completed,created_at\n");
    for item in &collection.items {
        let escaped_title = spodeian_export::escape_csv_field(&item.title);
        let escaped_desc = spodeian_export::escape_csv_field(&item.description);
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            spodeian_export::escape_csv_field(&item.id),
            escaped_title,
            escaped_desc,
            item.priority.label(),
            item.completed,
            item.created_at
        ));
    }
    Ok(csv)
}

/// Deserializes an item collection from a CSV string.
pub fn import_from_csv(csv_str: &str) -> Result<ItemCollection, ExportError> {
    let trimmed = csv_str.trim();
    if trimmed.is_empty() {
        return Err(ExportError::EmptyData);
    }

    let mut items = Vec::new();
    let lines = trimmed.lines().enumerate();

    for (idx, line) in lines {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        if idx == 0 && trimmed_line.to_lowercase().starts_with("id,") {
            continue;
        }

        let fields = spodeian_export::parse_csv_line(trimmed_line);
        if fields.len() < 5 {
            return Err(ExportError::Csv(format!(
                "Line {}: Expected at least 5 columns (id,title,description,priority,completed[,created_at]), found {}",
                idx + 1,
                fields.len()
            )));
        }

        let id = fields[0].clone();
        let title = fields[1].clone();
        let description = fields[2].clone();
        let priority = match fields[3].to_lowercase().as_str() {
            "low" => Priority::Low,
            "medium" => Priority::Medium,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            other => return Err(ExportError::InvalidPriority(other.to_string())),
        };
        let completed = fields[4].eq_ignore_ascii_case("true") || fields[4] == "1";
        let created_at = fields
            .get(5)
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        items.push(Item {
            id,
            title,
            description,
            priority,
            completed,
            created_at,
        });
    }

    if items.is_empty() {
        return Err(ExportError::Csv(
            "No valid items parsed from CSV".to_string(),
        ));
    }

    Ok(ItemCollection { items })
}

/// Exports the application state into compressed BSON binary bytes (Zlib-compressed BSON).
pub fn export_to_compressed_bson(state: &AppState) -> Result<Vec<u8>, String> {
    spodeian_export::export_to_compressed_bson(state)
        .map_err(|e| format!("BSON serialization failed: {e}"))
}

/// Imports and restores an AppState from a compressed (or raw) BSON slice.
pub fn import_from_compressed_bson(bytes: &[u8]) -> Result<AppState, String> {
    if bytes.is_empty() {
        return Err("BSON input is empty".to_string());
    }
    let bson_bytes =
        miniz_oxide::inflate::decompress_to_vec_zlib(bytes).unwrap_or_else(|_| bytes.to_vec());
    if let Ok(state) = bson::from_slice::<AppState>(&bson_bytes) {
        return Ok(state);
    }
    if let Ok(col) = bson::from_slice::<ItemCollection>(&bson_bytes) {
        return Ok(AppState {
            collection: col,
            config: AppConfig::default(),
        });
    }
    Err("BSON deserialization failed: incompatible payload".to_string())
}
