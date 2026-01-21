//! Token-aware merging for chunkers.
//!
//! This module provides functions to merge text segments based on token counts,
//! equivalent to Chonkie's Cython `merge.pyx`. Used by RecursiveChunker and
//! other chunkers that need to respect token limits.

/// Find merge indices for combining segments within token limits.
///
/// This is the core algorithm used by chunkers to find optimal merge points
/// based on token counts and chunk size limits.
///
/// # Arguments
///
/// * `token_counts` - Token count for each segment
/// * `chunk_size` - Maximum tokens per merged chunk
///
/// # Returns
///
/// Vector of end indices where merges should occur. Each index marks the
/// exclusive end of a merged chunk.
///
/// # Example
///
/// ```
/// use chunk::find_merge_indices;
///
/// let token_counts = vec![10, 15, 20, 5, 8, 12];
/// let indices = find_merge_indices(&token_counts, 30);
/// // Merge [0:2], [2:4], [4:6] -> indices = [2, 4, 6]
/// ```
pub fn find_merge_indices(token_counts: &[usize], chunk_size: usize) -> Vec<usize> {
    if token_counts.is_empty() {
        return vec![];
    }

    let n = token_counts.len();

    // Build cumulative token counts
    let mut cumulative: Vec<usize> = Vec::with_capacity(n + 1);
    cumulative.push(0);

    let mut sum = 0usize;
    for &count in token_counts {
        sum += count;
        cumulative.push(sum);
    }

    // Find merge indices using binary search
    let mut indices = Vec::new();
    let mut current_pos = 0;

    while current_pos < n {
        // Binary search for rightmost valid position
        let mut left = current_pos + 1;
        let mut right = n + 1;

        while left < right {
            let mid = (left + right) / 2;
            let fits = cumulative[mid] - cumulative[current_pos] <= chunk_size;

            if fits {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        // left is now one past the last valid position
        let index = left.saturating_sub(1).max(current_pos + 1).min(n);

        indices.push(index);
        current_pos = index;
    }

    indices
}

/// Compute merged token counts from merge indices.
fn compute_merged_token_counts(token_counts: &[usize], merge_indices: &[usize]) -> Vec<usize> {
    if merge_indices.is_empty() {
        return vec![];
    }

    let mut result = Vec::with_capacity(merge_indices.len());
    let mut start = 0;

    for &end in merge_indices {
        let end = end.min(token_counts.len());
        let sum: usize = token_counts[start..end].iter().sum();
        result.push(sum);
        start = end;
    }

    result
}

/// Result of merge_splits operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResult {
    /// Merged text segments
    pub merged: Vec<String>,
    /// Token count for each merged chunk
    pub token_counts: Vec<usize>,
}

/// Merge text segments based on token counts, respecting chunk size limits.
///
/// This is the Rust equivalent of Chonkie's Cython `_merge_splits` function.
/// Performs string concatenation in Rust for optimal performance.
///
/// # Arguments
///
/// * `splits` - Text segments to merge
/// * `token_counts` - Token count for each segment
/// * `chunk_size` - Maximum tokens per merged chunk
///
/// # Returns
///
/// `MergeResult` containing merged text and token counts.
///
/// # Example
///
/// ```
/// use chunk::merge_splits;
///
/// let splits = vec!["Hello", "world", "!", "How", "are", "you"];
/// let token_counts = vec![1, 1, 1, 1, 1, 1];
/// let result = merge_splits(&splits, &token_counts, 3);
/// assert_eq!(result.merged, vec!["Helloworld!", "Howareyou"]);
/// assert_eq!(result.token_counts, vec![3, 3]);
/// ```
pub fn merge_splits(
    splits: &[&str],
    token_counts: &[usize],
    chunk_size: usize,
) -> MergeResult {
    // Early exit for empty input
    if splits.is_empty() || token_counts.is_empty() {
        return MergeResult {
            merged: vec![],
            token_counts: vec![],
        };
    }

    // If all token counts exceed chunk_size, return segments as-is
    if token_counts.iter().all(|&c| c > chunk_size) {
        return MergeResult {
            merged: splits.iter().map(|s| s.to_string()).collect(),
            token_counts: token_counts.to_vec(),
        };
    }

    let indices = find_merge_indices(token_counts, chunk_size);
    let merged_counts = compute_merged_token_counts(token_counts, &indices);

    // Build merged strings
    let mut merged = Vec::with_capacity(indices.len());
    let mut start = 0;

    for &end in &indices {
        let end = end.min(splits.len());
        // Pre-calculate total length for efficient allocation
        let total_len: usize = splits[start..end].iter().map(|s| s.len()).sum();
        let mut s = String::with_capacity(total_len);
        for segment in &splits[start..end] {
            s.push_str(segment);
        }
        merged.push(s);
        start = end;
    }

    MergeResult {
        merged,
        token_counts: merged_counts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_merge_indices_basic() {
        let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
        let indices = find_merge_indices(&token_counts, 3);
        assert_eq!(indices, vec![3, 6, 7]);
    }

    #[test]
    fn test_find_merge_indices_large_chunks() {
        let token_counts = vec![10, 15, 20, 5, 8, 12];
        let indices = find_merge_indices(&token_counts, 30);
        assert_eq!(indices, vec![2, 4, 6]);
    }

    #[test]
    fn test_find_merge_indices_empty() {
        let token_counts: Vec<usize> = vec![];
        let indices = find_merge_indices(&token_counts, 10);
        assert!(indices.is_empty());
    }

    #[test]
    fn test_find_merge_indices_single() {
        let token_counts = vec![5];
        let indices = find_merge_indices(&token_counts, 10);
        assert_eq!(indices, vec![1]);
    }

    #[test]
    fn test_find_merge_indices_all_large() {
        let token_counts = vec![50, 60, 70];
        let indices = find_merge_indices(&token_counts, 30);
        assert_eq!(indices, vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_splits_basic() {
        let splits = vec!["a", "b", "c", "d", "e", "f", "g"];
        let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
        let result = merge_splits(&splits, &token_counts, 3);
        assert_eq!(result.merged, vec!["abc", "def", "g"]);
        assert_eq!(result.token_counts, vec![3, 3, 1]);
    }

    #[test]
    fn test_merge_splits_empty() {
        let splits: Vec<&str> = vec![];
        let token_counts: Vec<usize> = vec![];
        let result = merge_splits(&splits, &token_counts, 10);
        assert!(result.merged.is_empty());
        assert!(result.token_counts.is_empty());
    }

    #[test]
    fn test_merge_splits_all_exceed_limit() {
        let splits = vec!["aaa", "bbb", "ccc"];
        let token_counts = vec![50, 60, 70];
        let result = merge_splits(&splits, &token_counts, 30);
        assert_eq!(result.merged, vec!["aaa", "bbb", "ccc"]);
        assert_eq!(result.token_counts, vec![50, 60, 70]);
    }
}
