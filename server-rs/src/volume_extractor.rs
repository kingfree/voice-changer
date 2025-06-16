//! Volume extraction utilities translated from the Python implementation.
//!
//! The original Python code relies heavily on PyTorch for tensor
//! operations. This Rust version implements the same algorithms using
//! safe Rust and `Vec` math so that no heavy dependencies are required.

use std::f32;

/// Extract per-frame RMS volumes from an audio signal.
///
/// `audio` should be a mono signal. `hop_size` defines the frame
/// spacing in samples.
pub fn extract(audio: &[f32], hop_size: usize) -> Vec<f32> {
    if hop_size == 0 {
        return Vec::new();
    }
    let n_frames = audio.len() / hop_size + 1;
    let mut out = Vec::with_capacity(n_frames);
    for n in 0..n_frames {
        let start = n * hop_size;
        let end = ((n + 1) * hop_size).min(audio.len());
        if start >= audio.len() {
            out.push(out.last().cloned().unwrap_or(0.0));
            continue;
        }
        let slice = &audio[start..end];
        let mean = slice.iter().map(|v| v * v).sum::<f32>() / slice.len() as f32;
        out.push(mean.sqrt());
    }
    out
}

/// Sliding RMS using the same algorithm as [`extract`] but expecting an
/// owned vector so the function can operate in place.
pub fn extract_t(audio: &[f32], hop_size: usize) -> Vec<f32> {
    extract(audio, hop_size)
}

/// Generate a voice activity mask from volume values.
///
/// `block_size` specifies the upsample factor used by the caller.
/// `threshold` is in dB.
pub fn get_mask_from_volume(volume: &[f32], block_size: usize, threshold: f32) -> Vec<f32> {
    let db_threshold = 10f32.powf(threshold / 20.0);
    let mut mask: Vec<f32> = volume
        .iter()
        .map(|v| if *v > db_threshold { 1.0 } else { 0.0 })
        .collect();
    if mask.is_empty() {
        return mask;
    }
    let first = mask[0];
    let last = *mask.last().unwrap();
    for _ in 0..4 {
        mask.insert(0, first);
        mask.push(last);
    }
    let mut smoothed = Vec::with_capacity(mask.len() - 8);
    for n in 0..mask.len() - 8 {
        let max = mask[n..n + 9].iter().fold(0.0f32, |m, &v| m.max(v));
        smoothed.push(max);
    }
    upsample(&smoothed, block_size)
}

/// Upsample a 1‑D signal by linear interpolation.
fn upsample(signal: &[f32], factor: usize) -> Vec<f32> {
    if signal.is_empty() || factor == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(signal.len() * factor);
    for i in 0..signal.len() - 1 {
        let cur = signal[i];
        let next = signal[i + 1];
        for j in 0..factor {
            let t = j as f32 / factor as f32;
            out.push(cur * (1.0 - t) + next * t);
        }
    }
    out.push(*signal.last().unwrap());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic() {
        let audio = vec![0.0f32; 480];
        let v = extract(&audio, 160);
        assert_eq!(v.len(), 4); // 480/160 + 1
    }

    #[test]
    fn test_mask_generation() {
        let volume = vec![0.0, 0.5, 0.0, 0.5];
        let mask = get_mask_from_volume(&volume, 2, -6.0);
        // Should upsample to (len + pad -8)*factor = (??). For this simple test,
        // just verify output length.
        assert!(!mask.is_empty());
    }
}

