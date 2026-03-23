//! Model tier registry, on-disk discovery, integrity verification

use std::path::Path;

use serde::Serialize;

use crate::error::AppError;

pub struct ModelTier {
    pub id: &'static str,
    pub name: &'static str,
    pub file_name: &'static str,
    pub size_bytes: u64,
    pub description: &'static str,
    pub url: &'static str,
}

pub const MODEL_TIERS: &[ModelTier] = &[
    ModelTier {
        id: "large-v3",
        name: "Large (v3)",
        file_name: "ggml-large-v3-q5_0.bin",
        size_bytes: 1_080_000_000,
        description: "Best accuracy, recommended for most users",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin",
    },
    ModelTier {
        id: "small",
        name: "Small",
        file_name: "ggml-small-q5_1.bin",
        size_bytes: 200_000_000,
        description: "Good accuracy, lighter alternative",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
    },
    ModelTier {
        id: "base",
        name: "Base",
        file_name: "ggml-base-q5_1.bin",
        size_bytes: 60_000_000,
        description: "Minimal, for constrained hardware",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
    },
];

pub fn get_model_tier(id: &str) -> Option<&'static ModelTier> {
    MODEL_TIERS.iter().find(|m| m.id == id)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub description: String,
    pub downloaded: bool,
    pub file_size_on_disk: Option<u64>,
}

pub fn list_models(models_dir: &Path) -> Vec<ModelStatus> {
    MODEL_TIERS
        .iter()
        .map(|tier| {
            let path = models_dir.join(tier.file_name);
            let downloaded = path.exists();
            let file_size_on_disk = if downloaded {
                std::fs::metadata(&path).ok().map(|m| m.len())
            } else {
                None
            };
            ModelStatus {
                id: tier.id.to_string(),
                name: tier.name.to_string(),
                file_name: tier.file_name.to_string(),
                size_bytes: tier.size_bytes,
                description: tier.description.to_string(),
                downloaded,
                file_size_on_disk,
            }
        })
        .collect()
}

pub fn verify_model(models_dir: &Path, model_id: &str) -> Result<bool, AppError> {
    let tier = get_model_tier(model_id)
        .ok_or_else(|| AppError::Transcription(format!("Unknown model: {model_id}")))?;
    let path = models_dir.join(tier.file_name);
    if !path.exists() {
        return Ok(false);
    }
    let actual_size = std::fs::metadata(&path)?.len();
    let expected = tier.size_bytes;
    Ok(actual_size > expected * 95 / 100 && actual_size < expected * 105 / 100)
}

pub fn delete_model(models_dir: &Path, model_id: &str) -> Result<(), AppError> {
    let tier = get_model_tier(model_id)
        .ok_or_else(|| AppError::Transcription(format!("Unknown model: {model_id}")))?;
    let path = models_dir.join(tier.file_name);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
