//! Storage-agnostic domain layer for Amemory Skills, Books, Documents, and Memory.
//!
//! This crate contains pure domain models, frontmatter and path validation,
//! document inspection/text extraction, and deterministic chunk generation.
//! It has zero dependency on `xerj-*`, `rmcp`, or `axum`.

pub mod book;
pub mod document;
pub mod memory;
pub mod skill;

use thiserror::Error;

/// Structured domain error covering validation, lookup, conflict, and filesystem failures.
#[derive(Debug, Error)]
pub enum DomainError {
    /// Input validation or frontmatter/path constraint failure.
    #[error("{0}")]
    Validation(String),

    /// Requested Skill, Book, or resource file was not found.
    #[error("{0}")]
    NotFound(String),

    /// Target Skill or Book already exists on disk.
    #[error("{0}")]
    AlreadyExists(String),

    /// Underlying filesystem I/O failure.
    #[error("文件系统操作失败: {0}")]
    Io(#[from] std::io::Error),
}
