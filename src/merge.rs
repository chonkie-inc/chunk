//! Token-aware merging for chunkers.
//!
//! This module provides functions to merge text segments based on token counts,
//! equivalent to Chonkie's Cython `merge.pyx`. Used by RecursiveChunker and
//! other chunkers that need to respect token limits.

/// Find merge indices for combining segments within token limits.
///
/// This is the core algorithm used by RecursiveChunker to merge small segments
/// into larger chunks that fit within a token budget.
///
/// # Arguments
///
/// * `token_counts` - Token count for each segment
/// * `chunk_size` - Maximum tokens per merged chunk
/// * `combine_whitespace` - If true, adds +1 token per join for whitespace
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
/// let indices = find_merge_indices(&token_counts, 30, false);
/// // Merge [0:2], [2:4], [4:6] -> indices = [2, 4, 6]
/// ```
pub fn find_merge_indices(
    token_counts: &[usize],
    chunk_size: usize,
    combine_whitespace: bool,
) -> Vec<usize> {
    if token_counts.is_empty() {
        return vec![];
    }

    let n = token_counts.len();

    // Build cumulative token counts (raw, without whitespace adjustment)
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
        // For a chunk from current_pos to end:
        // - Raw tokens: cumulative[end] - cumulative[current_pos]
        // - Whitespace tokens (if combine_whitespace): (end - current_pos - 1) for joins
        // - Total must be <= chunk_size
        //
        // With whitespace: cumulative[end] + end <= cumulative[current_pos] + current_pos + chunk_size + 1
        // Without:         cumulative[end] <= cumulative[current_pos] + chunk_size

        // Binary search for rightmost valid position
        let mut left = current_pos + 1;
        let mut right = n + 1;

        while left < right {
            let mid = (left + right) / 2;
            let fits = if combine_whitespace {
                // Total tokens = raw_sum + (end - start - 1) whitespace joins
                let raw_sum = cumulative[mid] - cumulative[current_pos];
                let whitespace = if mid > current_pos + 1 {
                    mid - current_pos - 1
                } else {
                    0
                };
                raw_sum + whitespace <= chunk_size
            } else {
                cumulative[mid] - cumulative[current_pos] <= chunk_size
            };

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
///
/// Given the original token counts and merge indices from `find_merge_indices`,
/// compute the token count for each merged chunk.
///
/// # Arguments
///
/// * `token_counts` - Original token counts for each segment
/// * `merge_indices` - End indices from `find_merge_indices`
/// * `combine_whitespace` - If true, adds +1 token per join for whitespace
///
/// # Returns
///
/// Vector of token counts for each merged chunk.
pub fn compute_merged_token_counts(
    token_counts: &[usize],
    merge_indices: &[usize],
    combine_whitespace: bool,
) -> Vec<usize> {
    if merge_indices.is_empty() {
        return vec![];
    }

    let mut result = Vec::with_capacity(merge_indices.len());
    let mut start = 0;

    for &end in merge_indices {
        let end = end.min(token_counts.len());
        let mut sum: usize = token_counts[start..end].iter().sum();

        if combine_whitespace && end > start {
            // Add whitespace tokens for joins (n-1 joins for n segments)
            sum += end - start - 1;
        }

        result.push(sum);
        start = end;
    }

    result
}

/// Result of merge_splits operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResult {
    /// End indices for each merged chunk (exclusive).
    /// Use with slicing: segments[prev_end..end]
    pub indices: Vec<usize>,
    /// Token count for each merged chunk
    pub token_counts: Vec<usize>,
}

/// Merge segments based on token counts, respecting chunk size limits.
///
/// This is the Rust equivalent of Chonkie's Cython `_merge_splits` function.
/// Returns indices for slicing the original segments, rather than copying strings.
///
/// # Arguments
///
/// * `token_counts` - Token count for each segment
/// * `chunk_size` - Maximum tokens per merged chunk
/// * `combine_whitespace` - If true, join with whitespace (+1 token per join)
///
/// # Returns
///
/// `MergeResult` containing:
/// - `indices`: End indices for slicing segments
/// - `token_counts`: Token count for each merged chunk
///
/// # Example
///
/// ```
/// use chunk::merge_splits;
///
/// // segments = ["Hello", "world", "!", "How", "are", "you", "?"]
/// let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
/// let result = merge_splits(&token_counts, 5, true);
///
/// // Use indices to slice: segments[0..3], segments[3..6], segments[6..7]
/// // chunk_size=5 allows 3 segments + 2 whitespace joins = 5 tokens per chunk
/// assert_eq!(result.indices, vec![3, 6, 7]);
/// assert_eq!(result.token_counts, vec![5, 5, 1]); // includes whitespace tokens
/// ```
pub fn merge_splits(
    token_counts: &[usize],
    chunk_size: usize,
    combine_whitespace: bool,
) -> MergeResult {
    // Early exit for empty input
    if token_counts.is_empty() {
        return MergeResult {
            indices: vec![],
            token_counts: vec![],
        };
    }

    // If all token counts exceed chunk_size, return one chunk per segment
    if token_counts.iter().all(|&c| c > chunk_size) {
        let indices: Vec<usize> = (1..=token_counts.len()).collect();
        return MergeResult {
            indices,
            token_counts: token_counts.to_vec(),
        };
    }

    let indices = find_merge_indices(token_counts, chunk_size, combine_whitespace);
    let merged_counts = compute_merged_token_counts(token_counts, &indices, combine_whitespace);

    MergeResult {
        indices,
        token_counts: merged_counts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_merge_indices_basic() {
        let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
        let indices = find_merge_indices(&token_counts, 3, false);
        // Should merge into groups of 3 tokens
        assert_eq!(indices, vec![3, 6, 7]);
    }

    #[test]
    fn test_find_merge_indices_with_whitespace() {
        let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
        let indices = find_merge_indices(&token_counts, 3, true);
        // With whitespace: (n-1) joins add (n-1) whitespace tokens
        // Chunk 0..2: raw=2, whitespace=1, total=3 <= 3. Fits.
        // Chunk 2..4: raw=2, whitespace=1, total=3 <= 3. Fits.
        // Chunk 4..6: raw=2, whitespace=1, total=3 <= 3. Fits.
        // Chunk 6..7: raw=1, whitespace=0, total=1 <= 3. Fits.
        assert_eq!(indices, vec![2, 4, 6, 7]);
    }

    #[test]
    fn test_find_merge_indices_large_chunks() {
        let token_counts = vec![10, 15, 20, 5, 8, 12];
        let indices = find_merge_indices(&token_counts, 30, false);
        // 10+15=25 < 30, 10+15+20=45 > 30 -> merge at 2
        // 20 < 30, 20+5=25 < 30, 20+5+8=33 > 30 -> merge at 4
        // 8+12=20 < 30 -> merge at 6
        assert_eq!(indices, vec![2, 4, 6]);
    }

    #[test]
    fn test_find_merge_indices_empty() {
        let token_counts: Vec<usize> = vec![];
        let indices = find_merge_indices(&token_counts, 10, false);
        assert!(indices.is_empty());
    }

    #[test]
    fn test_find_merge_indices_single() {
        let token_counts = vec![5];
        let indices = find_merge_indices(&token_counts, 10, false);
        assert_eq!(indices, vec![1]);
    }

    #[test]
    fn test_find_merge_indices_all_large() {
        // All segments exceed chunk_size
        let token_counts = vec![50, 60, 70];
        let indices = find_merge_indices(&token_counts, 30, false);
        // Each segment becomes its own chunk
        assert_eq!(indices, vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_splits_basic() {
        let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
        let result = merge_splits(&token_counts, 3, false);
        assert_eq!(result.indices, vec![3, 6, 7]);
        assert_eq!(result.token_counts, vec![3, 3, 1]);
    }

    #[test]
    fn test_merge_splits_with_whitespace() {
        let token_counts = vec![1, 1, 1, 1, 1, 1, 1];
        let result = merge_splits(&token_counts, 5, true);
        // With whitespace tokens added for joins
        // [1,1] + 1 whitespace = 3, [1,1] + 1 = 3, etc.
        assert_eq!(result.indices, vec![3, 6, 7]);
        assert_eq!(result.token_counts, vec![5, 5, 1]);
    }

    #[test]
    fn test_merge_splits_empty() {
        let token_counts: Vec<usize> = vec![];
        let result = merge_splits(&token_counts, 10, false);
        assert!(result.indices.is_empty());
        assert!(result.token_counts.is_empty());
    }

    #[test]
    fn test_merge_splits_all_exceed_limit() {
        let token_counts = vec![50, 60, 70];
        let result = merge_splits(&token_counts, 30, false);
        assert_eq!(result.indices, vec![1, 2, 3]);
        assert_eq!(result.token_counts, vec![50, 60, 70]);
    }

    #[test]
    fn test_compute_merged_token_counts() {
        let token_counts = vec![10, 15, 20, 5, 8, 12];
        let indices = vec![2, 4, 6];
        let merged = compute_merged_token_counts(&token_counts, &indices, false);
        assert_eq!(merged, vec![25, 25, 20]); // 10+15, 20+5, 8+12
    }

    #[test]
    fn test_compute_merged_token_counts_with_whitespace() {
        let token_counts = vec![1, 1, 1];
        let indices = vec![3];
        let merged = compute_merged_token_counts(&token_counts, &indices, true);
        // 1+1+1 + 2 whitespace tokens = 5
        assert_eq!(merged, vec![5]);
    }
}
