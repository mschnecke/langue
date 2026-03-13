//! Opus/OGG encoding with sinc resampling, WAV fallback

use crate::error::AppError;
use audiopus::{coder::Encoder, Application, Channels, SampleRate};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::io::Cursor;

/// Encode PCM samples to Opus in an Ogg container.
pub fn encode_to_opus(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, AppError> {
    let (resampled, target_rate) = resample_for_opus(samples, sample_rate, channels)?;

    // Convert f32 to i16
    let samples_i16: Vec<i16> = resampled
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            (clamped * 32767.0) as i16
        })
        .collect();

    let opus_channels = if channels == 1 {
        Channels::Mono
    } else {
        Channels::Stereo
    };

    let opus_sample_rate = match target_rate {
        8000 => SampleRate::Hz8000,
        12000 => SampleRate::Hz12000,
        16000 => SampleRate::Hz16000,
        24000 => SampleRate::Hz24000,
        _ => SampleRate::Hz48000,
    };

    let mut encoder = Encoder::new(opus_sample_rate, opus_channels, Application::Voip)
        .map_err(|e| AppError::Audio(format!("Failed to create Opus encoder: {}", e)))?;

    // 24kbps bitrate — good quality for voice, small file size
    encoder
        .set_bitrate(audiopus::Bitrate::BitsPerSecond(24000))
        .map_err(|e| AppError::Audio(format!("Failed to set bitrate: {}", e)))?;

    // 20ms frames
    let frame_size = (target_rate as usize) / 50;

    let mut encoded_frames: Vec<Vec<u8>> = Vec::new();

    for chunk in samples_i16.chunks(frame_size * channels as usize) {
        let frame_to_encode = if chunk.len() < frame_size * channels as usize {
            let mut padded = chunk.to_vec();
            padded.resize(frame_size * channels as usize, 0);
            padded
        } else {
            chunk.to_vec()
        };

        let mut buffer = vec![0u8; 4000];
        let encoded_len = encoder
            .encode(&frame_to_encode, &mut buffer)
            .map_err(|e| AppError::Audio(format!("Opus encoding failed: {}", e)))?;
        encoded_frames.push(buffer[..encoded_len].to_vec());
    }

    let ogg_data = wrap_in_ogg(&encoded_frames, target_rate, channels, frame_size)?;
    Ok(ogg_data)
}

/// Resample audio to an Opus-compatible rate using high-quality sinc interpolation.
fn resample_for_opus(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<(Vec<f32>, u32), AppError> {
    let target_rate = match sample_rate {
        r if r <= 8000 => 8000,
        r if r <= 12000 => 12000,
        r if r <= 16000 => 16000,
        r if r <= 24000 => 24000,
        _ => 48000,
    };

    if sample_rate == target_rate {
        return Ok((samples.to_vec(), target_rate));
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let num_channels = channels as usize;

    // Deinterleave into per-channel vectors
    let samples_per_channel = samples.len() / num_channels;
    let mut channel_data: Vec<Vec<f32>> =
        vec![Vec::with_capacity(samples_per_channel); num_channels];

    for (i, &sample) in samples.iter().enumerate() {
        channel_data[i % num_channels].push(sample);
    }

    let mut resampler = SincFixedIn::<f32>::new(
        target_rate as f64 / sample_rate as f64,
        2.0,
        params,
        channel_data[0].len(),
        num_channels,
    )
    .map_err(|e| AppError::Audio(format!("Failed to create resampler: {}", e)))?;

    let resampled_channels = resampler
        .process(&channel_data, None)
        .map_err(|e| AppError::Audio(format!("Failed to resample audio: {}", e)))?;

    // Interleave channels back together
    let resampled_len = resampled_channels[0].len();
    let mut output = Vec::with_capacity(resampled_len * num_channels);

    for i in 0..resampled_len {
        for channel in &resampled_channels {
            output.push(channel[i]);
        }
    }

    Ok((output, target_rate))
}

/// Wrap Opus frames in an Ogg container with proper headers.
fn wrap_in_ogg(
    encoded_frames: &[Vec<u8>],
    sample_rate: u32,
    channels: u16,
    frame_size: usize,
) -> Result<Vec<u8>, AppError> {
    use ogg::writing::PacketWriter;

    let mut output = Vec::new();
    let mut cursor = Cursor::new(&mut output);

    {
        let mut writer = PacketWriter::new(&mut cursor);
        let serial = 0u32;

        // OpusHead identification header
        let mut id_header = Vec::new();
        id_header.extend_from_slice(b"OpusHead");
        id_header.push(1); // Version
        id_header.push(channels as u8);
        id_header.extend_from_slice(&312u16.to_le_bytes()); // Pre-skip
        id_header.extend_from_slice(&sample_rate.to_le_bytes()); // Input sample rate
        id_header.extend_from_slice(&0i16.to_le_bytes()); // Output gain
        id_header.push(0); // Channel mapping family

        writer
            .write_packet(
                id_header,
                serial,
                ogg::writing::PacketWriteEndInfo::EndPage,
                0,
            )
            .map_err(|e| AppError::Audio(format!("Failed to write Opus header: {}", e)))?;

        // OpusTags comment header
        let mut comment_header = Vec::new();
        comment_header.extend_from_slice(b"OpusTags");
        let vendor = b"pisum-langue";
        comment_header.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        comment_header.extend_from_slice(vendor);
        comment_header.extend_from_slice(&0u32.to_le_bytes()); // No user comments

        writer
            .write_packet(
                comment_header,
                serial,
                ogg::writing::PacketWriteEndInfo::EndPage,
                0,
            )
            .map_err(|e| AppError::Audio(format!("Failed to write Opus comment: {}", e)))?;

        // Audio frames with granule positions (48kHz-relative)
        let samples_per_frame = frame_size as u64;
        let granule_increment = samples_per_frame * 48000 / sample_rate as u64;

        for (i, frame) in encoded_frames.iter().enumerate() {
            let granule_pos = (i as u64 + 1) * granule_increment;
            let is_last = i == encoded_frames.len() - 1;

            let end_info = if is_last {
                ogg::writing::PacketWriteEndInfo::EndStream
            } else {
                ogg::writing::PacketWriteEndInfo::NormalPacket
            };

            writer
                .write_packet(frame.clone(), serial, end_info, granule_pos)
                .map_err(|e| AppError::Audio(format!("Failed to write Opus frame: {}", e)))?;
        }
    }

    Ok(output)
}

/// Encode PCM samples to WAV format (fallback).
pub fn encode_to_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, AppError> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| AppError::Audio(format!("Failed to create WAV writer: {}", e)))?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let sample_i16 = (clamped * 32767.0) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| AppError::Audio(format!("Failed to write WAV sample: {}", e)))?;
        }

        writer
            .finalize()
            .map_err(|e| AppError::Audio(format!("Failed to finalize WAV: {}", e)))?;
    }

    Ok(cursor.into_inner())
}

/// MIME type for Opus audio
pub fn opus_mime_type() -> &'static str {
    "audio/ogg"
}

/// MIME type for WAV audio
pub fn wav_mime_type() -> &'static str {
    "audio/wav"
}
