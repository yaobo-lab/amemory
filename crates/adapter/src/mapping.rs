//! Explicit XERJ index schema and document source mapping for Amemory indices.

use domain::memory::{IndexDocument, IndexTarget};
use serde_json::{Map, Value};
use xerj_common::types::{EmbeddingConfig, FieldConfig, FieldOptions, FieldType, Schema};

use crate::AdapterError;

/// Default dimension for XERJ's built-in server-side lexical semantic embedder.
pub const DEFAULT_SEMANTIC_DIMENSIONS: usize = 256;

/// Build the explicit XERJ [`Schema`] for the given [`IndexTarget`].
///
/// # Errors
///
/// Returns [`AdapterError::Schema`] if adding a field to the schema fails.
pub fn build_index_schema(target: IndexTarget) -> Result<Schema, AdapterError> {
    let mut schema = Schema::empty();

    add_keyword(&mut schema, "doc_id")?;
    add_text(&mut schema, "title")?;
    add_keyword(&mut schema, "uri")?;
    add_text(&mut schema, "text")?;
    add_semantic_text(&mut schema, "semantic_text", "semantic_text_vector")?;
    add_keyword(&mut schema, "tags")?;
    add_keyword(&mut schema, "source_kind")?;
    add_keyword(&mut schema, "owner_name")?;
    add_keyword(&mut schema, "relative_path")?;
    add_long(&mut schema, "part_index")?;
    add_long(&mut schema, "part_total")?;
    add_keyword(&mut schema, "content_hash")?;
    add_date(&mut schema, "updated_at")?;

    match target {
        IndexTarget::Skills => {
            add_keyword(&mut schema, "skill_name")?;
            add_text(&mut schema, "skill_description")?;
        }
        IndexTarget::Books => {
            add_keyword(&mut schema, "book_name")?;
            add_text(&mut schema, "book_description")?;
            add_keyword(&mut schema, "document_file_name")?;
            add_keyword(&mut schema, "document_format")?;
        }
        IndexTarget::Memory => {
            add_keyword(&mut schema, "namespace")?;
        }
    }

    Ok(schema)
}

/// Convert an [`IndexDocument`] into the JSON `_source` object stored in XERJ.
#[must_use]
pub fn document_to_source(doc: &IndexDocument) -> Value {
    let mut map = Map::new();
    map.insert("doc_id".to_string(), Value::String(doc.doc_id.clone()));
    map.insert("title".to_string(), Value::String(doc.title.clone()));
    map.insert("uri".to_string(), Value::String(doc.uri.clone()));
    map.insert("text".to_string(), Value::String(doc.text.clone()));
    map.insert(
        "semantic_text".to_string(),
        Value::String(format!("{}\n{}", doc.title, doc.text)),
    );
    map.insert(
        "tags".to_string(),
        Value::Array(doc.tags.iter().cloned().map(Value::String).collect()),
    );
    map.insert(
        "source_kind".to_string(),
        Value::String(doc.source_kind.clone()),
    );
    map.insert(
        "owner_name".to_string(),
        Value::String(doc.owner_name.clone()),
    );
    map.insert(
        "relative_path".to_string(),
        Value::String(doc.relative_path.clone()),
    );
    map.insert(
        "part_index".to_string(),
        Value::Number(serde_json::Number::from(doc.part_index)),
    );
    map.insert(
        "part_total".to_string(),
        Value::Number(serde_json::Number::from(doc.part_total)),
    );
    map.insert(
        "content_hash".to_string(),
        Value::String(doc.content_hash.clone()),
    );
    map.insert(
        "updated_at".to_string(),
        Value::String(doc.updated_at.clone()),
    );

    for (key, value) in &doc.extra {
        map.insert(key.clone(), value.clone());
    }

    Value::Object(map)
}

fn add_text(schema: &mut Schema, name: &str) -> Result<(), AdapterError> {
    schema
        .add_field(FieldConfig::new(name, FieldType::Text))
        .map_err(|e| AdapterError::Schema(e.to_string()))
}

fn add_keyword(schema: &mut Schema, name: &str) -> Result<(), AdapterError> {
    let options = FieldOptions {
        analyzed: false,
        term_positions: false,
        ..FieldOptions::default()
    };
    schema
        .add_field(FieldConfig::new(name, FieldType::Keyword).with_options(options))
        .map_err(|e| AdapterError::Schema(e.to_string()))
}

fn add_long(schema: &mut Schema, name: &str) -> Result<(), AdapterError> {
    let options = FieldOptions {
        analyzed: false,
        term_positions: false,
        ..FieldOptions::default()
    };
    schema
        .add_field(FieldConfig::new(name, FieldType::Long).with_options(options))
        .map_err(|e| AdapterError::Schema(e.to_string()))
}

fn add_date(schema: &mut Schema, name: &str) -> Result<(), AdapterError> {
    schema
        .add_field(FieldConfig::new(name, FieldType::Date))
        .map_err(|e| AdapterError::Schema(e.to_string()))
}

fn add_semantic_text(
    schema: &mut Schema,
    name: &str,
    companion_name: &str,
) -> Result<(), AdapterError> {
    let mut field = FieldConfig::new(name, FieldType::Text);
    field.options.dimensions = Some(DEFAULT_SEMANTIC_DIMENSIONS);
    field.options.similarity = Some("cosine".to_string());
    field.embedding = Some(EmbeddingConfig {
        endpoint: None,
        model: None,
        target_field: Some(companion_name.to_string()),
    });
    schema
        .add_field(field)
        .map_err(|e| AdapterError::Schema(e.to_string()))?;

    let mut companion = FieldConfig::new(companion_name, FieldType::Vector);
    companion.options.dimensions = Some(DEFAULT_SEMANTIC_DIMENSIONS);
    companion.options.similarity = Some("cosine".to_string());
    schema
        .add_field(companion)
        .map_err(|e| AdapterError::Schema(e.to_string()))
}
