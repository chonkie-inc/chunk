//! Savitzky-Golay filter and related signal processing functions.
//!
//! This module provides optimized implementations for:
//! - Savitzky-Golay filtering for smoothing and derivatives
//! - Local minima detection with interpolation
//! - Windowed cross-similarity for semantic chunking
//! - Split index filtering by percentile threshold

/// Result of finding local minima.
#[derive(Debug, Clone)]
pub struct MinimaResult {
    pub indices: Vec<usize>,
    pub values: Vec<f64>,
}

/// Result of filtering split indices.
#[derive(Debug, Clone)]
pub struct FilteredIndices {
    pub indices: Vec<usize>,
    pub values: Vec<f64>,
}

// =============================================================================
// Matrix Operations
// =============================================================================

/// Multiply matrices A (m x n) and B (n x p), storing result in C (m x p).
fn matrix_multiply(a: &[f64], b: &[f64], m: usize, n: usize, p: usize) -> Vec<f64> {
    let mut c = vec![0.0; m * p];
    for i in 0..m {
        for j in 0..p {
            let mut sum = 0.0;
            for k in 0..n {
                sum += a[i * n + k] * b[k * p + j];
            }
            c[i * p + j] = sum;
        }
    }
    c
}

/// Transpose matrix A (m x n) to AT (n x m).
fn matrix_transpose(a: &[f64], m: usize, n: usize) -> Vec<f64> {
    let mut at = vec![0.0; n * m];
    for i in 0..m {
        for j in 0..n {
            at[j * m + i] = a[i * n + j];
        }
    }
    at
}

/// Invert matrix A (n x n) using Gaussian elimination with partial pivoting.
/// Returns None if the matrix is singular.
fn matrix_inverse(a: &[f64], n: usize) -> Option<Vec<f64>> {
    // Create identity matrix
    let mut a_inv = vec![0.0; n * n];
    for i in 0..n {
        a_inv[i * n + i] = 1.0;
    }

    // Working copy of A
    let mut work = a.to_vec();

    // Gaussian elimination with partial pivoting
    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        let mut max_val = work[i * n + i].abs();
        for k in (i + 1)..n {
            let val = work[k * n + i].abs();
            if val > max_val {
                max_val = val;
                max_row = k;
            }
        }

        // Swap rows if needed
        if max_row != i {
            for j in 0..n {
                work.swap(i * n + j, max_row * n + j);
                a_inv.swap(i * n + j, max_row * n + j);
            }
        }

        // Check for singular matrix
        let pivot = work[i * n + i];
        if pivot.abs() < 1e-10 {
            return None;
        }

        // Normalize pivot row
        for j in 0..n {
            work[i * n + j] /= pivot;
            a_inv[i * n + j] /= pivot;
        }

        // Eliminate column
        for k in 0..n {
            if k != i {
                let factor = work[k * n + i];
                for j in 0..n {
                    work[k * n + j] -= factor * work[i * n + j];
                    a_inv[k * n + j] -= factor * a_inv[i * n + j];
                }
            }
        }
    }

    Some(a_inv)
}

// =============================================================================
// Savitzky-Golay Filter
// =============================================================================

/// Compute Savitzky-Golay filter coefficients.
///
/// # Arguments
/// * `window_size` - Size of the filter window (must be odd)
/// * `poly_order` - Order of the polynomial
/// * `deriv` - Derivative order (0 = smoothing)
fn compute_savgol_coeffs(window_size: usize, poly_order: usize, deriv: usize) -> Option<Vec<f64>> {
    let half_window = (window_size - 1) / 2;
    let poly_cols = poly_order + 1;

    // Build Vandermonde matrix A (window_size x poly_cols)
    let mut a = vec![0.0; window_size * poly_cols];
    for i in 0..window_size {
        let x = i as f64 - half_window as f64;
        for j in 0..poly_cols {
            a[i * poly_cols + j] = x.powi(j as i32);
        }
    }

    // Compute A^T
    let at = matrix_transpose(&a, window_size, poly_cols);

    // Compute A^T * A
    let ata = matrix_multiply(&at, &a, poly_cols, window_size, poly_cols);

    // Invert (A^T * A)
    let ata_inv = matrix_inverse(&ata, poly_cols)?;

    // Compute factorial for derivative
    let factorial: f64 = (1..=deriv).map(|i| i as f64).product::<f64>().max(1.0);

    // Extract coefficients for the requested derivative
    let mut coeffs = vec![0.0; window_size];
    for i in 0..window_size {
        if deriv < poly_cols {
            let mut sum = 0.0;
            for k in 0..poly_cols {
                sum += ata_inv[deriv * poly_cols + k] * a[i * poly_cols + k];
            }
            coeffs[i] = factorial * sum;
        }
    }

    Some(coeffs)
}

/// Apply convolution with boundary reflection.
fn apply_convolution(data: &[f64], kernel: &[f64]) -> Vec<f64> {
    let n = data.len();
    let kernel_size = kernel.len();
    let half = kernel_size / 2;
    let mut output = vec![0.0; n];

    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..kernel_size {
            let mut idx = i as isize - half as isize + j as isize;
            // Reflect at boundaries
            if idx < 0 {
                idx = -idx;
            } else if idx >= n as isize {
                idx = 2 * n as isize - idx - 2;
            }
            // Clamp to valid range
            idx = idx.clamp(0, n as isize - 1);
            sum += data[idx as usize] * kernel[j];
        }
        output[i] = sum;
    }

    output
}

/// Apply Savitzky-Golay filter to data.
///
/// # Arguments
/// * `data` - Input signal
/// * `window_length` - Filter window length (must be odd and > poly_order)
/// * `poly_order` - Polynomial order for fitting
/// * `deriv` - Derivative order (0 = smoothing, 1 = first derivative, 2 = second)
///
/// # Returns
/// Filtered data or None if parameters are invalid.
pub fn savgol_filter(
    data: &[f64],
    window_length: usize,
    poly_order: usize,
    deriv: usize,
) -> Option<Vec<f64>> {
    if window_length % 2 == 0 || window_length <= poly_order || data.is_empty() {
        return None;
    }

    let coeffs = compute_savgol_coeffs(window_length, poly_order, deriv)?;
    Some(apply_convolution(data, &coeffs))
}

// =============================================================================
// Local Minima Detection
// =============================================================================

/// Find local minima using first and second derivatives from Savitzky-Golay filter.
///
/// A point is considered a minimum if:
/// - First derivative is near zero (within tolerance)
/// - Second derivative is positive (concave up)
///
/// # Arguments
/// * `data` - Input signal
/// * `window_size` - Savitzky-Golay window size (must be odd)
/// * `poly_order` - Polynomial order
/// * `tolerance` - Tolerance for considering first derivative as zero
///
/// # Returns
/// MinimaResult with indices and values of local minima.
pub fn find_local_minima_interpolated(
    data: &[f64],
    window_size: usize,
    poly_order: usize,
    tolerance: f64,
) -> Option<MinimaResult> {
    if data.is_empty() {
        return Some(MinimaResult {
            indices: vec![],
            values: vec![],
        });
    }

    // Get first and second derivatives
    let first_deriv = savgol_filter(data, window_size, poly_order, 1)?;
    let second_deriv = savgol_filter(data, window_size, poly_order, 2)?;

    // Find minima
    let mut indices = Vec::new();
    let mut values = Vec::new();

    for i in 0..data.len() {
        if first_deriv[i].abs() < tolerance && second_deriv[i] > 0.0 {
            indices.push(i);
            values.push(data[i]);
        }
    }

    Some(MinimaResult { indices, values })
}

// =============================================================================
// Windowed Cross-Similarity
// =============================================================================

/// Compute windowed cross-similarity for semantic chunking.
///
/// For each position, computes the average cosine similarity between consecutive
/// embeddings within a sliding window.
///
/// # Arguments
/// * `embeddings` - Flattened 2D array of embeddings (n_sentences x embedding_dim)
/// * `n` - Number of embeddings (sentences)
/// * `d` - Embedding dimension
/// * `window_size` - Size of sliding window (must be odd and >= 3)
///
/// # Returns
/// Vector of average similarities (length n-1) or None if parameters are invalid.
pub fn windowed_cross_similarity(
    embeddings: &[f64],
    n: usize,
    d: usize,
    window_size: usize,
) -> Option<Vec<f64>> {
    if window_size % 2 == 0 || window_size < 3 || n < 2 || d == 0 {
        return None;
    }

    let half_window = window_size / 2;
    let mut result = vec![0.0; n - 1];

    for i in 0..(n - 1) {
        // Define window boundaries
        let start = i.saturating_sub(half_window);
        let end = (i + half_window + 2).min(n);

        // Calculate average similarity in window
        let mut total_sim = 0.0;
        let mut count = 0;

        for j in start..(end - 1) {
            // Compute cosine similarity between consecutive embeddings
            let emb1_start = j * d;
            let emb2_start = (j + 1) * d;

            let mut dot = 0.0;
            let mut norm1 = 0.0;
            let mut norm2 = 0.0;

            for k in 0..d {
                let v1 = embeddings[emb1_start + k];
                let v2 = embeddings[emb2_start + k];
                dot += v1 * v2;
                norm1 += v1 * v1;
                norm2 += v2 * v2;
            }

            if norm1 > 0.0 && norm2 > 0.0 {
                total_sim += dot / (norm1.sqrt() * norm2.sqrt());
                count += 1;
            }
        }

        result[i] = if count > 0 {
            total_sim / count as f64
        } else {
            0.0
        };
    }

    Some(result)
}

// =============================================================================
// Split Index Filtering
// =============================================================================

/// Calculate percentile of a slice.
fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx = p * (sorted.len() - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = (lower + 1).min(sorted.len() - 1);
    let weight = idx - lower as f64;

    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}

/// Filter split indices by percentile threshold and minimum distance.
///
/// # Arguments
/// * `indices` - Candidate split indices
/// * `values` - Values at those indices
/// * `threshold` - Percentile threshold (0.0-1.0)
/// * `min_distance` - Minimum distance between splits
///
/// # Returns
/// FilteredIndices with indices and values that pass the filter.
pub fn filter_split_indices(
    indices: &[usize],
    values: &[f64],
    threshold: f64,
    min_distance: usize,
) -> FilteredIndices {
    if indices.is_empty() || values.is_empty() {
        return FilteredIndices {
            indices: vec![],
            values: vec![],
        };
    }

    // Calculate threshold value
    let threshold_val = percentile(values, threshold);

    // Filter indices
    let mut result_indices = Vec::new();
    let mut result_values = Vec::new();
    let mut last_idx: Option<usize> = None;

    for (&idx, &val) in indices.iter().zip(values.iter()) {
        let distance_ok = match last_idx {
            Some(last) => idx >= last + min_distance,
            None => true,
        };

        if val <= threshold_val && distance_ok {
            result_indices.push(idx);
            result_values.push(val);
            last_idx = Some(idx);
        }
    }

    FilteredIndices {
        indices: result_indices,
        values: result_values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_savgol_filter_smoothing() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let result = savgol_filter(&data, 5, 2, 0).unwrap();
        assert_eq!(result.len(), data.len());
        // Linear data should remain approximately linear after smoothing
        for (i, &val) in result.iter().enumerate() {
            assert!((val - (i as f64 + 1.0)).abs() < 0.5);
        }
    }

    #[test]
    fn test_savgol_filter_invalid_params() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        // Even window size
        assert!(savgol_filter(&data, 4, 2, 0).is_none());
        // Window size <= poly_order
        assert!(savgol_filter(&data, 3, 3, 0).is_none());
    }

    #[test]
    fn test_find_local_minima() {
        // Create data with a clear minimum
        let data: Vec<f64> = (0..20)
            .map(|i| {
                let x = (i as f64 - 10.0) / 3.0;
                x * x // Parabola with minimum at i=10
            })
            .collect();

        let result = find_local_minima_interpolated(&data, 5, 2, 0.5).unwrap();
        // Should find minimum near index 10
        assert!(!result.indices.is_empty());
        let min_idx = result.indices[0];
        assert!((min_idx as isize - 10).abs() <= 2);
    }

    #[test]
    fn test_windowed_cross_similarity() {
        // Simple test with 3 identical embeddings
        let embeddings = vec![
            1.0, 0.0, 0.0, // emb 1
            1.0, 0.0, 0.0, // emb 2 (same as 1)
            0.0, 1.0, 0.0, // emb 3 (orthogonal)
        ];
        let result = windowed_cross_similarity(&embeddings, 3, 3, 3).unwrap();
        assert_eq!(result.len(), 2);
        // First similarity (1-2) should be 1.0
        assert!((result[0] - 0.5).abs() < 0.1); // Window average
    }

    #[test]
    fn test_filter_split_indices() {
        let indices = vec![0, 5, 8, 15, 20];
        let values = vec![0.1, 0.3, 0.2, 0.5, 0.4];

        // Filter with 50th percentile threshold and min distance 3
        let result = filter_split_indices(&indices, &values, 0.5, 3);
        // Should keep indices with values <= 0.3 (50th percentile) and min distance 3
        assert!(!result.indices.is_empty());
    }

    #[test]
    fn test_percentile() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&data, 0.0) - 1.0).abs() < 0.001);
        assert!((percentile(&data, 0.5) - 3.0).abs() < 0.001);
        assert!((percentile(&data, 1.0) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_matrix_inverse() {
        // 2x2 identity matrix
        let a = vec![1.0, 0.0, 0.0, 1.0];
        let inv = matrix_inverse(&a, 2).unwrap();
        assert!((inv[0] - 1.0).abs() < 0.001);
        assert!((inv[3] - 1.0).abs() < 0.001);

        // Simple 2x2 matrix [[2, 1], [1, 1]] -> inverse [[1, -1], [-1, 2]]
        let b = vec![2.0, 1.0, 1.0, 1.0];
        let inv_b = matrix_inverse(&b, 2).unwrap();
        assert!((inv_b[0] - 1.0).abs() < 0.001);
        assert!((inv_b[1] - (-1.0)).abs() < 0.001);
        assert!((inv_b[2] - (-1.0)).abs() < 0.001);
        assert!((inv_b[3] - 2.0).abs() < 0.001);
    }
}
