//! TranscriptionProvider trait definition

use std::future::Future;
use std::pin::Pin;

use crate::error::AppError;

/// Result from a transcription provider
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
}

/// Trait for AI transcription providers (dyn-compatible via boxed futures)
pub trait TranscriptionProvider: Send + Sync {
    /// Transcribe audio data using the given system prompt
    fn transcribe(
        &self,
        audio_data: &[u8],
        mime_type: &str,
        system_prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TranscriptionResult, AppError>> + Send + '_>>;

    /// Test the connection / API key validity
    fn test_connection(&self) -> Pin<Box<dyn Future<Output = Result<bool, AppError>> + Send + '_>>;

    /// Provider display name
    fn provider_name(&self) -> &str;
}
