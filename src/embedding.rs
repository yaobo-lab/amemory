//! 构造用于 Book 文档导入的可共享文本嵌入提供方。

use std::sync::Arc;

use memvid_core::{EmbeddingProvider, OpenAIConfig, OpenAIEmbedder};
#[cfg(feature = "local-embedding")]
use memvid_core::{LocalTextEmbedder, TextEmbedConfig};

use crate::config::{EmbeddingConfig, EmbeddingProviderMode};

/// 可在线程间共享的 Memvid 文本嵌入提供方。
pub type SharedEmbedder = Arc<dyn EmbeddingProvider>;

/// 根据运行配置创建文档嵌入提供方。
///
/// # Errors
///
/// 本地模型配置无效，或 `OpenAI` API key 环境变量缺失时返回错误。
pub fn create(config: &EmbeddingConfig) -> eyre::Result<Option<SharedEmbedder>> {
    let provider: Option<SharedEmbedder> = match config.provider {
        EmbeddingProviderMode::None => None,
        EmbeddingProviderMode::Local => create_local(config)?,
        EmbeddingProviderMode::Openai => {
            let mut openai_config = OpenAIConfig::default()
                .with_api_key_env(&config.api_key_env)
                .with_base_url(&config.base_url)
                .with_timeout(config.timeout_secs);
            openai_config.model.clone_from(&config.openai_model);
            openai_config.max_retries = config.max_retries;
            Some(Arc::new(OpenAIEmbedder::new(openai_config)?))
        }
    };
    Ok(provider)
}

#[cfg(feature = "local-embedding")]
fn create_local(config: &EmbeddingConfig) -> eyre::Result<Option<SharedEmbedder>> {
    let local_config = TextEmbedConfig {
        model_name: config.local_model.clone(),
        models_dir: config.models_dir.clone(),

        ..TextEmbedConfig::default()
    };
    Ok(Some(Arc::new(LocalTextEmbedder::new(local_config)?)))
}

#[cfg(not(feature = "local-embedding"))]
fn create_local(_config: &EmbeddingConfig) -> eyre::Result<Option<SharedEmbedder>> {
    eyre::bail!(
        "embedding.provider=local requires rebuilding memvid-mcp with --features local-embedding"
    )
}
