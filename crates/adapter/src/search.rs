//! Search query translation and hit normalization over XERJ indices.

use amemory_domain::memory::{SearchHit, SearchMode, SearchRequest};
use serde_json::{Value, json};
use xerj_engine::Engine;
use xerj_query::parse_request;

use crate::AdapterError;

/// Execute a unified [`SearchRequest`] across the configured Amemory indices.
///
/// # Errors
///
/// Returns [`AdapterError`] if query parsing or index search execution fails.
pub async fn execute_search(
    engine: &Engine,
    skills_index: &str,
    books_index: &str,
    memory_index: &str,
    request: &SearchRequest,
) -> Result<Vec<SearchHit>, AdapterError> {
    let target_indices = select_target_indices(
        skills_index,
        books_index,
        memory_index,
        request.uri.as_deref(),
        request.scope.as_deref(),
    );

    let fetch_size = (request.limit.saturating_mul(3)).clamp(10, 200);
    let body = build_search_body(&request.query, request.mode, fetch_size);
    let xerj_req =
        parse_request(&body).map_err(|e| AdapterError::Query(format!("解析搜索请求失败: {e}")))?;

    let mut candidates = Vec::new();
    for index_name in target_indices {
        let Ok(idx) = engine.get_index(index_name) else {
            continue;
        };
        let result = Box::pin(idx.search(&xerj_req))
            .await
            .map_err(|e| AdapterError::Engine(e.to_string()))?;

        for hit in result.hits {
            if let Some(mapped) = map_and_filter_hit(index_name, hit, request) {
                candidates.push(mapped);
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .total_cmp(&a.score.unwrap_or(0.0))
            .then_with(|| a.doc_id.cmp(&b.doc_id))
    });
    candidates.truncate(request.limit);
    Ok(candidates)
}

fn select_target_indices<'a>(
    skills_index: &'a str,
    books_index: &'a str,
    memory_index: &'a str,
    uri: Option<&str>,
    scope: Option<&str>,
) -> Vec<&'a str> {
    let hint = uri.or(scope).unwrap_or("");
    if hint.starts_with("skill://") {
        vec![skills_index]
    } else if hint.starts_with("book://") {
        vec![books_index]
    } else if hint.starts_with("memory://") {
        vec![memory_index]
    } else {
        vec![skills_index, books_index, memory_index]
    }
}

fn build_search_body(query: &str, mode: SearchMode, size: usize) -> Value {
    let lexical_clause = json!({
        "multi_match": {
            "query": query,
            "fields": ["title^3", "text^2", "skill_description^2", "book_description^2", "owner_name^2", "relative_path"]
        }
    });

    let semantic_clause = json!({
        "semantic": {
            "field": "semantic_text",
            "query": query,
            "k": size
        }
    });

    let query_node = match mode {
        SearchMode::Lexical => lexical_clause,
        SearchMode::Semantic => semantic_clause,
        SearchMode::Hybrid => json!({
            "hybrid": {
                "fusion": "rrf",
                "queries": [
                    { "query": lexical_clause, "weight": 1.0 },
                    { "query": semantic_clause, "weight": 1.0 }
                ]
            }
        }),
    };

    json!({
        "size": size,
        "query": query_node
    })
}

fn map_and_filter_hit(
    index_name: &str,
    hit: xerj_query::Hit,
    request: &SearchRequest,
) -> Option<SearchHit> {
    let obj = hit.source.as_object()?;
    let uri = obj.get("uri").and_then(Value::as_str).map(str::to_string);
    let title = obj.get("title").and_then(Value::as_str).map(str::to_string);
    let full_text = obj.get("text").and_then(Value::as_str).unwrap_or_default();
    let tags: Vec<String> = obj
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    if let Some(exact_uri) = &request.uri
        && uri.as_deref() != Some(exact_uri.as_str())
    {
        return None;
    }

    if let Some(scope_prefix) = &request.scope
        && !uri
            .as_deref()
            .is_some_and(|u| u.starts_with(scope_prefix.as_str()))
    {
        return None;
    }

    if let Some(u) = uri.as_deref() {
        if request.exclude_uris.iter().any(|ex| ex == u) {
            return None;
        }
        if request
            .exclude_scopes
            .iter()
            .any(|prefix| u.starts_with(prefix.as_str()))
        {
            return None;
        }
    }

    for filter_tag in &request.tag_filters {
        if !tags.iter().any(|tag| tag.eq_ignore_ascii_case(filter_tag)) {
            return None;
        }
    }

    let snippet = truncate_snippet(full_text, request.snippet_chars);
    let mut meta = serde_json::Map::new();
    for key in [
        "source_kind",
        "owner_name",
        "relative_path",
        "part_index",
        "part_total",
        "content_hash",
        "updated_at",
        "skill_name",
        "skill_description",
        "book_name",
        "book_description",
        "document_file_name",
        "document_format",
        "namespace",
        "metadata",
    ] {
        if let Some(val) = obj.get(key) {
            meta.insert(key.to_string(), val.clone());
        }
    }

    Some(SearchHit {
        doc_id: hit.id,
        index: index_name.to_string(),
        score: Some(hit.score),
        title,
        uri,
        text: snippet,
        tags,
        metadata: Value::Object(meta),
    })
}

fn truncate_snippet(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let prefix: String = trimmed.chars().take(max_chars).collect();
    format!("{prefix}…")
}
