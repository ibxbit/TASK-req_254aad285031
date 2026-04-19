// Pure-Rust image analysis helpers for the face-data module.
// Deterministic (no randomness); no external ML / cloud services.
//
// Metrics stored per image:
//   brightness_score = normalized mean luma, in [0, 1]
//   blur_score       = variance of the Laplacian (higher = sharper)
//   perceptual_hash  = 64-bit DCT-based pHash (hex)
//
// The single-frontal-face check (no always-pass fallback) uses a skin-tone
// density map + centroid symmetry heuristic: we require exactly one
// sufficiently-dense skin region, centered horizontally and symmetric
// around the vertical axis. This is local, fast, and rejects the common
// failure modes (multiple faces, off-center side profile, background
// noise) without any external model.

use image::{imageops, DynamicImage, GenericImageView};
use sha2::{Digest, Sha256};

// --- Thresholds ---

pub const MIN_WIDTH: u32 = 320;
pub const MIN_HEIGHT: u32 = 320;

/// Normalized brightness range (mean luma / 255).
pub const MIN_BRIGHTNESS: f64 = 0.3;
pub const MAX_BRIGHTNESS: f64 = 0.8;

/// Minimum Laplacian variance for acceptable sharpness.
pub const MIN_LAPLACIAN_VARIANCE: f64 = 100.0;

/// Dedup threshold: reject on Hamming distance ≤ this many bits (out of 64).
pub const DEDUP_HAMMING_THRESHOLD: u32 = 5;

/// Fraction of pixels in a central crop that must be skin-toned for a
/// single face to be considered present (and not just background).
pub const MIN_FACE_DENSITY: f64 = 0.10;

/// Max horizontal deviation of the skin centroid from the frame center,
/// as a fraction of width. Larger values allow off-center subjects; the
/// spec requires frontal framing.
pub const MAX_CENTER_OFFSET: f64 = 0.20;

/// Minimum left/right symmetry ratio of skin density (0.0 = fully skewed,
/// 1.0 = perfect symmetry).
pub const MIN_LR_SYMMETRY: f64 = 0.60;

/// When the skin regions break into distinct horizontal clusters this far
/// apart (fraction of width), we treat it as "multiple faces" and reject.
pub const MULTI_FACE_GAP_FRACTION: f64 = 0.25;

// --- Content hash (SHA-256) ---

pub fn content_hash_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

// --- Perceptual hash (DCT-based pHash) ---

/// 64-bit DCT perceptual hash.
///
/// Algorithm:
///   1) grayscale, resize to 32x32
///   2) 2D DCT
///   3) keep top-left 8x8 low-frequency block
///   4) compare each of the 64 coeffs to the median of the block
///      (DC at [0,0] is excluded from the median computation)
pub fn perceptual_hash(img: &DynamicImage) -> u64 {
    let gray = img.to_luma8();
    let small = imageops::resize(&gray, 32, 32, imageops::FilterType::Lanczos3);

    let mut matrix = [[0f64; 32]; 32];
    for y in 0..32u32 {
        for x in 0..32u32 {
            matrix[y as usize][x as usize] = small.get_pixel(x, y)[0] as f64;
        }
    }

    let dct = dct_2d_32(&matrix);

    let mut coeffs = [0f64; 64];
    for y in 0..8 {
        for x in 0..8 {
            coeffs[y * 8 + x] = dct[y][x];
        }
    }

    // Median over coeffs[1..] (exclude DC term)
    let mut tail: Vec<f64> = coeffs[1..].to_vec();
    tail.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = tail[tail.len() / 2];

    let mut hash: u64 = 0;
    for (i, &c) in coeffs.iter().enumerate() {
        if c > median {
            hash |= 1u64 << i;
        }
    }
    hash
}

pub fn perceptual_hash_hex(img: &DynamicImage) -> String {
    format!("{:016x}", perceptual_hash(img))
}

pub fn parse_perceptual_hex(s: &str) -> Option<u64> {
    u64::from_str_radix(s, 16).ok()
}

pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

// Separable 2D DCT on a 32x32 block (naive O(n^3) per axis; adequate for 32).
fn dct_2d_32(m: &[[f64; 32]; 32]) -> [[f64; 32]; 32] {
    let mut rows = [[0f64; 32]; 32];
    for y in 0..32 {
        rows[y] = dct_1d_32(&m[y]);
    }
    let mut out = [[0f64; 32]; 32];
    for x in 0..32 {
        let mut col = [0f64; 32];
        for y in 0..32 {
            col[y] = rows[y][x];
        }
        let col_dct = dct_1d_32(&col);
        for y in 0..32 {
            out[y][x] = col_dct[y];
        }
    }
    out
}

fn dct_1d_32(input: &[f64; 32]) -> [f64; 32] {
    const N: usize = 32;
    let factor = std::f64::consts::PI / (2.0 * N as f64);
    let mut out = [0f64; N];
    for k in 0..N {
        let mut sum = 0.0;
        for (i, &v) in input.iter().enumerate() {
            sum += v * ((2 * i + 1) as f64 * k as f64 * factor).cos();
        }
        out[k] = sum;
    }
    out
}

// --- Metrics ---

/// Normalized mean luma in [0, 1].
pub fn compute_brightness(img: &DynamicImage) -> f64 {
    let gray = img.to_luma8();
    let (w, h) = img.dimensions();
    let n = (w as u64 * h as u64) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let sum: u64 = gray.pixels().map(|p| p[0] as u64).sum();
    (sum as f64 / n) / 255.0
}

/// Variance of the Laplacian. Higher = sharper image, lower = blurrier.
///
/// Kernel (4-neighbor):
///   0 -1  0
///  -1  4 -1
///   0 -1  0
pub fn compute_laplacian_variance(img: &DynamicImage) -> f64 {
    let gray = img.to_luma8();
    let (w, h) = gray.dimensions();
    if w < 3 || h < 3 {
        return 0.0;
    }

    let mut lap: Vec<f64> = Vec::with_capacity(((w - 2) * (h - 2)) as usize);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let c = gray.get_pixel(x, y)[0] as f64;
            let u = gray.get_pixel(x, y - 1)[0] as f64;
            let d = gray.get_pixel(x, y + 1)[0] as f64;
            let l = gray.get_pixel(x - 1, y)[0] as f64;
            let r = gray.get_pixel(x + 1, y)[0] as f64;
            lap.push(4.0 * c - u - d - l - r);
        }
    }
    if lap.is_empty() {
        return 0.0;
    }
    let mean: f64 = lap.iter().sum::<f64>() / lap.len() as f64;
    lap.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / lap.len() as f64
}

/// Convenience wrapper returning (brightness_normalized, laplacian_variance).
pub fn compute_metrics(img: &DynamicImage) -> (f64, f64) {
    (compute_brightness(img), compute_laplacian_variance(img))
}

// --- Single frontal face detection (heuristic, local, deterministic) ---

#[derive(Debug, Clone)]
pub struct FrontalFaceAnalysis {
    pub skin_density: f64,
    pub center_offset: f64,
    pub lr_symmetry: f64,
    pub cluster_count: u32,
}

/// Heuristic detector suitable for an offline/on-prem build. Returns the
/// underlying analysis so the API response is auditable.
///
/// Strategy:
///   1. Convert to YCbCr and threshold Cr/Cb to a common skin-tone range.
///   2. Compute density of skin pixels in the central 60% of the frame.
///   3. Compute horizontal centroid and left/right half density ratio.
///   4. Split the skin mask into column bands; require one dominant cluster.
pub fn analyze_frontal_face(img: &DynamicImage) -> FrontalFaceAnalysis {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    if w == 0 || h == 0 {
        return FrontalFaceAnalysis {
            skin_density: 0.0,
            center_offset: 1.0,
            lr_symmetry: 0.0,
            cluster_count: 0,
        };
    }

    // Central 60% horizontal band and 80% vertical band: excludes edges
    // where hands/arms / background clutter commonly register as skin.
    let x0 = (w as f64 * 0.20) as u32;
    let x1 = (w as f64 * 0.80).min(w as f64) as u32;
    let y0 = (h as f64 * 0.10) as u32;
    let y1 = (h as f64 * 0.90).min(h as f64) as u32;

    let crop_w = (x1 - x0).max(1);
    let crop_h = (y1 - y0).max(1);
    let total = (crop_w as u64 * crop_h as u64) as f64;

    let mut skin_mask = vec![false; (crop_w * crop_h) as usize];
    let mut skin_count: u64 = 0;
    let mut sum_x: u64 = 0;
    let mut left_count: u64 = 0;
    let mut right_count: u64 = 0;

    let mid_x = crop_w / 2;

    for y in y0..y1 {
        for x in x0..x1 {
            let p = rgb.get_pixel(x, y);
            if is_skin_tone(p[0], p[1], p[2]) {
                let lx = x - x0;
                let ly = y - y0;
                skin_mask[(ly * crop_w + lx) as usize] = true;
                skin_count += 1;
                sum_x += lx as u64;
                if lx < mid_x {
                    left_count += 1;
                } else {
                    right_count += 1;
                }
            }
        }
    }

    let skin_density = skin_count as f64 / total;
    let center_offset = if skin_count == 0 {
        1.0
    } else {
        let centroid_x = sum_x as f64 / skin_count as f64;
        let crop_center = crop_w as f64 / 2.0;
        (centroid_x - crop_center).abs() / crop_w as f64
    };
    let lr_symmetry = {
        let a = left_count.min(right_count) as f64;
        let b = left_count.max(right_count) as f64;
        if b == 0.0 {
            0.0
        } else {
            a / b
        }
    };

    // Count horizontally-distinct skin clusters. Collapse the 2D mask into
    // per-column density, smooth, then count separated runs above 20% of
    // the max column density; gaps wider than MULTI_FACE_GAP_FRACTION mean
    // "two faces" -> reject.
    let mut col_density = vec![0u32; crop_w as usize];
    for (i, &present) in skin_mask.iter().enumerate() {
        if present {
            let col = i % crop_w as usize;
            col_density[col] += 1;
        }
    }
    let col_max = *col_density.iter().max().unwrap_or(&0) as f64;
    let col_threshold = (col_max * 0.20).max(1.0);

    let min_gap_cols = (crop_w as f64 * MULTI_FACE_GAP_FRACTION) as u32;
    let mut clusters = 0u32;
    let mut gap = 0u32;
    let mut in_cluster = false;
    for &d in col_density.iter() {
        if (d as f64) >= col_threshold {
            if !in_cluster {
                clusters += 1;
                in_cluster = true;
            }
            gap = 0;
        } else {
            gap += 1;
            if in_cluster && gap >= min_gap_cols {
                in_cluster = false;
            }
        }
    }

    FrontalFaceAnalysis {
        skin_density,
        center_offset,
        lr_symmetry,
        cluster_count: clusters,
    }
}

/// Skin-tone predicate in YCbCr space (from Hsu/Abdel-Mottaleb/Jain 2002,
/// widely used in offline face detection). Avoids illuminant sensitivity.
fn is_skin_tone(r: u8, g: u8, b: u8) -> bool {
    let rf = r as f64;
    let gf = g as f64;
    let bf = b as f64;
    // Y channel ignored here; Cb and Cr carry the chroma information.
    let cb = 128.0 - 0.168736 * rf - 0.331264 * gf + 0.5 * bf;
    let cr = 128.0 + 0.5 * rf - 0.418688 * gf - 0.081312 * bf;
    (77.0..=127.0).contains(&cb) && (133.0..=173.0).contains(&cr)
}

// --- Validation ---

pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub message: Option<String>,
}

pub fn run_checks(
    width: u32,
    height: u32,
    brightness_normalized: f64,
    laplacian_variance: f64,
    frontal: Option<&FrontalFaceAnalysis>,
) -> Vec<Check> {
    let mut out = Vec::new();

    let res_ok = width >= MIN_WIDTH && height >= MIN_HEIGHT;
    out.push(Check {
        name: "resolution",
        passed: res_ok,
        message: if res_ok {
            None
        } else {
            Some(format!(
                "must be at least {}x{}, got {}x{}",
                MIN_WIDTH, MIN_HEIGHT, width, height
            ))
        },
    });

    let b_ok = (MIN_BRIGHTNESS..=MAX_BRIGHTNESS).contains(&brightness_normalized);
    out.push(Check {
        name: "brightness",
        passed: b_ok,
        message: if b_ok {
            None
        } else {
            Some(format!(
                "normalized brightness {:.3} out of range [{:.2}, {:.2}]",
                brightness_normalized, MIN_BRIGHTNESS, MAX_BRIGHTNESS
            ))
        },
    });

    let blur_ok = laplacian_variance >= MIN_LAPLACIAN_VARIANCE;
    out.push(Check {
        name: "blur",
        passed: blur_ok,
        message: if blur_ok {
            None
        } else {
            Some(format!(
                "Laplacian variance {:.1} below threshold {:.0}",
                laplacian_variance, MIN_LAPLACIAN_VARIANCE
            ))
        },
    });

    // Single frontal face. NO always-pass fallback: when analysis is not
    // available (caller didn't run it) the check fails and the audit trail
    // records that. When analysis is present, all four sub-conditions must
    // hold for the check to pass.
    let (passed, msg) = match frontal {
        None => (
            false,
            Some("single_frontal_face analysis missing".to_string()),
        ),
        Some(f) => {
            let density_ok = f.skin_density >= MIN_FACE_DENSITY;
            let center_ok = f.center_offset <= MAX_CENTER_OFFSET;
            let sym_ok = f.lr_symmetry >= MIN_LR_SYMMETRY;
            let one_cluster_ok = f.cluster_count == 1;
            let ok = density_ok && center_ok && sym_ok && one_cluster_ok;
            let msg = if ok {
                None
            } else {
                Some(format!(
                    "density={:.2} (≥{:.2}), center_offset={:.2} (≤{:.2}), \
                     lr_symmetry={:.2} (≥{:.2}), clusters={} (=1)",
                    f.skin_density,
                    MIN_FACE_DENSITY,
                    f.center_offset,
                    MAX_CENTER_OFFSET,
                    f.lr_symmetry,
                    MIN_LR_SYMMETRY,
                    f.cluster_count,
                ))
            };
            (ok, msg)
        }
    };
    out.push(Check {
        name: "single_frontal_face",
        passed,
        message: msg,
    });

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_checks_fails_when_frontal_missing() {
        let checks = run_checks(320, 320, 0.5, 200.0, None);
        let frontal = checks
            .iter()
            .find(|c| c.name == "single_frontal_face")
            .unwrap();
        assert!(!frontal.passed, "must fail when analysis missing");
    }

    #[test]
    fn run_checks_fails_on_multi_face() {
        let a = FrontalFaceAnalysis {
            skin_density: 0.3,
            center_offset: 0.05,
            lr_symmetry: 0.9,
            cluster_count: 2,
        };
        let checks = run_checks(320, 320, 0.5, 200.0, Some(&a));
        let frontal = checks
            .iter()
            .find(|c| c.name == "single_frontal_face")
            .unwrap();
        assert!(!frontal.passed);
    }

    #[test]
    fn run_checks_fails_on_off_center() {
        let a = FrontalFaceAnalysis {
            skin_density: 0.3,
            center_offset: 0.4,
            lr_symmetry: 0.9,
            cluster_count: 1,
        };
        let checks = run_checks(320, 320, 0.5, 200.0, Some(&a));
        let frontal = checks
            .iter()
            .find(|c| c.name == "single_frontal_face")
            .unwrap();
        assert!(!frontal.passed);
    }

    #[test]
    fn run_checks_passes_good_frontal() {
        let a = FrontalFaceAnalysis {
            skin_density: 0.25,
            center_offset: 0.05,
            lr_symmetry: 0.85,
            cluster_count: 1,
        };
        let checks = run_checks(320, 320, 0.5, 200.0, Some(&a));
        assert!(checks.iter().all(|c| c.passed));
    }

    #[test]
    fn content_hash_is_stable() {
        let h1 = content_hash_hex(b"hello");
        let h2 = content_hash_hex(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn content_hash_differs_on_different_bytes() {
        assert_ne!(content_hash_hex(b"a"), content_hash_hex(b"b"));
    }
}
