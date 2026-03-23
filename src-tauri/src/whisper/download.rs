//! Model download with progress events and cancellation

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::error::AppError;

static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percentage: f64,
}

pub async fn download_model(
    app: &AppHandle,
    model_id: &str,
    models_dir: &std::path::Path,
) -> Result<(), AppError> {
    if DOWNLOAD_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return Err(AppError::ModelDownload(
            "A download is already in progress. Cancel it first.".into(),
        ));
    }
    DOWNLOAD_CANCELLED.store(false, Ordering::SeqCst);

    let result = do_download(app, model_id, models_dir).await;

    DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);

    if let Err(ref _e) = result {
        if let Some(tier) = super::models::get_model_tier(model_id) {
            let partial = models_dir.join(tier.file_name);
            let _ = std::fs::remove_file(&partial);
        }
    }

    result
}

async fn do_download(
    app: &AppHandle,
    model_id: &str,
    models_dir: &std::path::Path,
) -> Result<(), AppError> {
    let tier = super::models::get_model_tier(model_id)
        .ok_or_else(|| AppError::ModelDownload(format!("Unknown model: {model_id}")))?;

    std::fs::create_dir_all(models_dir)?;
    let dest = models_dir.join(tier.file_name);

    let client = reqwest::Client::new();
    let response = client
        .get(tier.url)
        .send()
        .await
        .map_err(|e| AppError::ModelDownload(format!("Download failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::ModelDownload(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(tier.size_bytes);
    let mut file = std::fs::File::create(&dest)?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        if DOWNLOAD_CANCELLED.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&dest);
            return Err(AppError::ModelDownload("Download cancelled".into()));
        }

        let chunk = chunk.map_err(|e| AppError::ModelDownload(format!("Network error: {e}")))?;
        std::io::Write::write_all(&mut file, &chunk)?;
        downloaded += chunk.len() as u64;

        if downloaded % (100 * 1024) < chunk.len() as u64 {
            let _ = app.emit(
                "whisper-download-progress",
                DownloadProgress {
                    model_id: model_id.to_string(),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                    percentage: (downloaded as f64 / total as f64) * 100.0,
                },
            );
        }
    }

    if !super::models::verify_model(models_dir, model_id)? {
        let _ = std::fs::remove_file(&dest);
        return Err(AppError::ModelDownload(
            "Downloaded model failed integrity check. Please retry.".into(),
        ));
    }

    Ok(())
}

pub fn cancel_download() {
    DOWNLOAD_CANCELLED.store(true, Ordering::SeqCst);
}
