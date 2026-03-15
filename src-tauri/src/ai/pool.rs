//! Round-robin provider pool with fallback

use std::sync::atomic::{AtomicUsize, Ordering};

use super::gemini::GeminiProvider;
use super::openai::OpenAiProvider;
use super::provider::{TranscriptionProvider, TranscriptionResult};
use crate::error::AppError;

/// Configuration for a single provider instance
pub struct ProviderEntry {
    pub api_key: String,
    pub model: Option<String>,
    pub provider_type: String,
}

/// Round-robin provider pool that distributes requests and falls back on failure
pub struct ProviderPool {
    providers: Vec<Box<dyn TranscriptionProvider>>,
    current_index: AtomicUsize,
}

impl ProviderPool {
    /// Create an empty pool
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            current_index: AtomicUsize::new(0),
        }
    }

    /// Rebuild the pool from provider configurations
    pub fn rebuild(&mut self, entries: &[ProviderEntry]) {
        self.providers.clear();
        self.current_index.store(0, Ordering::Relaxed);

        for entry in entries {
            match entry.provider_type.as_str() {
                "gemini" | "Gemini" => {
                    let provider =
                        GeminiProvider::new(entry.api_key.clone(), entry.model.clone());
                    self.providers.push(Box::new(provider));
                    tracing::info!("Added Gemini provider to pool");
                }
                "openai" | "OpenAi" => {
                    let provider =
                        OpenAiProvider::new(entry.api_key.clone(), entry.model.clone());
                    self.providers.push(Box::new(provider));
                    tracing::info!("Added OpenAI provider to pool");
                }
                other => {
                    tracing::warn!("Unknown provider type: {}, skipping", other);
                }
            }
        }

        tracing::info!("Provider pool rebuilt with {} providers", self.providers.len());
    }

    /// Transcribe audio using round-robin selection with fallback.
    /// Tries each provider in sequence starting from the current index.
    pub async fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        system_prompt: &str,
    ) -> Result<TranscriptionResult, AppError> {
        let len = self.providers.len();
        if len == 0 {
            return Err(AppError::Transcription(
                "No AI providers configured. Please add a provider in Settings.".to_string(),
            ));
        }

        let start = self.current_index.fetch_add(1, Ordering::Relaxed) % len;
        let mut errors = Vec::new();

        for i in 0..len {
            let idx = (start + i) % len;
            let provider = &self.providers[idx];

            match provider.transcribe(audio_data, mime_type, system_prompt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "Provider {} ({}) failed: {}",
                        idx,
                        provider.provider_name(),
                        e
                    );
                    errors.push(format!("{}: {}", provider.provider_name(), e));
                }
            }
        }

        Err(AppError::Transcription(format!(
            "All providers failed: {}",
            errors.join("; ")
        )))
    }

    /// Test connection for a specific provider config (not from the pool)
    pub async fn test_provider(entry: &ProviderEntry) -> Result<bool, AppError> {
        match entry.provider_type.as_str() {
            "gemini" | "Gemini" => {
                let provider =
                    GeminiProvider::new(entry.api_key.clone(), entry.model.clone());
                provider.test_connection().await
            }
            "openai" | "OpenAi" => {
                let provider =
                    OpenAiProvider::new(entry.api_key.clone(), entry.model.clone());
                provider.test_connection().await
            }
            other => Err(AppError::Transcription(format!(
                "Unknown provider type: {}",
                other
            ))),
        }
    }

    /// Check if the pool has any providers configured
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}
