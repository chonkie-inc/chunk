//! Shared delimiter utilities for chunking and splitting.
//!
//! This module contains the core delimiter search functions using
//! SIMD-accelerated memchr (1-3 delimiters) or lookup table (4+ delimiters).

use memchr::memmem;

/// Default chunk target size (4KB).
pub const DEFAULT_TARGET_SIZE: usize = 4096;

/// Default delimiters: newline, period, question mark.
pub const DEFAULT_DELIMITERS: &[u8] = b"\n.?";

/// Find last delimiter in window using SIMD-accelerated memchr (1-3 delimiters)
/// or lookup table (4+ delimiters).
#[inline]
pub fn find_last_delimiter(
    window: &[u8],
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
) -> Option<usize> {
    if let Some(t) = table {
        window.iter().rposition(|&b| t[b as usize])
    } else {
        match delimiters.len() {
            1 => memchr::memrchr(delimiters[0], window),
            2 => memchr::memrchr2(delimiters[0], delimiters[1], window),
            3 => memchr::memrchr3(delimiters[0], delimiters[1], delimiters[2], window),
            0 => None,
            _ => unreachable!(),
        }
    }
}

/// Find first delimiter in window using SIMD-accelerated memchr (1-3 delimiters)
/// or lookup table (4+ delimiters).
#[inline]
pub fn find_first_delimiter(
    window: &[u8],
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
) -> Option<usize> {
    if let Some(t) = table {
        window.iter().position(|&b| t[b as usize])
    } else {
        match delimiters.len() {
            1 => memchr::memchr(delimiters[0], window),
            2 => memchr::memchr2(delimiters[0], delimiters[1], window),
            3 => memchr::memchr3(delimiters[0], delimiters[1], delimiters[2], window),
            0 => None,
            _ => unreachable!(),
        }
    }
}

/// Build lookup table for 4+ delimiters.
#[inline]
pub fn build_table(delimiters: &[u8]) -> Option<[bool; 256]> {
    if delimiters.len() > 3 {
        let mut t = [false; 256];
        for &b in delimiters {
            t[b as usize] = true;
        }
        Some(t)
    } else {
        None
    }
}

/// Find delimiter boundary that is the START of a consecutive run.
///
/// Searches backward from `target_end`, then forward if `forward_fallback` is true.
/// When `consecutive` is true, returns position of a delimiter that is NOT preceded
/// by the same delimiter byte.
///
/// Returns positions > start (never returns start itself, as that wouldn't make progress).
pub fn find_delimiter_boundary(
    text: &[u8],
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
    start: usize,
    target_end: usize,
    consecutive: bool,
    forward_fallback: bool,
) -> Option<usize> {
    if delimiters.is_empty() || start >= text.len() {
        return None;
    }

    let target_end = target_end.min(text.len());
    let window = &text[start..target_end];

    // Backward search
    if consecutive {
        // Find last delimiter that is START of consecutive run (same delimiter)
        let mut search_end = window.len();
        while search_end > 0 {
            let rel_pos = find_last_delimiter(&window[..search_end], delimiters, table);

            if let Some(rel_pos) = rel_pos {
                let abs_pos = start + rel_pos;
                let delim_byte = text[abs_pos];
                // Check if this is START of consecutive run (not preceded by same delimiter)
                if abs_pos == 0 || text[abs_pos - 1] != delim_byte {
                    // Found valid boundary, but skip if it equals start (no progress)
                    if abs_pos > start {
                        return Some(abs_pos);
                    }
                    // We've traced back to start - chunk is full of consecutive delimiters
                    // Fall through to forward fallback
                    break;
                }
                // In middle of run, search earlier
                search_end = rel_pos;
            } else {
                break;
            }
        }
    } else {
        // Simple case: just find last occurrence (but not at start position)
        let rel_pos = find_last_delimiter(window, delimiters, table);
        if let Some(rel_pos) = rel_pos {
            let abs_pos = start + rel_pos;
            if abs_pos > start {
                return Some(abs_pos);
            }
        }
    }

    // Forward fallback search - find next boundary after target_end
    if forward_fallback {
        // First, determine where to start searching forward.
        // If we're at the start of a consecutive run, skip past the entire run.
        let mut forward_from = target_end;

        if consecutive && start < text.len() {
            // Check if start is at a delimiter
            let is_delim_at_start = if let Some(t) = table {
                t[text[start] as usize]
            } else {
                delimiters.contains(&text[start])
            };

            if is_delim_at_start {
                // We're at a delimiter at `start`. Skip all consecutive same delimiters.
                let delim_byte = text[start];
                let mut pos = start;
                while pos < text.len() && text[pos] == delim_byte {
                    pos += 1;
                }
                // Start searching from end of consecutive run, but not before target_end
                forward_from = forward_from.max(pos);
            }
        }

        if forward_from < text.len() {
            let forward_window = &text[forward_from..];

            if consecutive {
                // Find first delimiter that is START of consecutive run
                let mut search_start = 0;
                while search_start < forward_window.len() {
                    let rel_pos =
                        find_first_delimiter(&forward_window[search_start..], delimiters, table);

                    if let Some(rel_pos) = rel_pos {
                        let abs_pos = forward_from + search_start + rel_pos;
                        let delim_byte = text[abs_pos];
                        if abs_pos == 0 || text[abs_pos - 1] != delim_byte {
                            return Some(abs_pos);
                        }
                        // In middle of run, search later
                        search_start += rel_pos + 1;
                    } else {
                        break;
                    }
                }
            } else {
                let rel_pos = find_first_delimiter(forward_window, delimiters, table);
                if let Some(rel_pos) = rel_pos {
                    return Some(forward_from + rel_pos);
                }
            }
        }

        // No delimiter found forward. Return text.len() to include all remaining
        // text in one chunk. This avoids O(n²) behavior from repeatedly searching
        // forward through the entire remaining text on each iteration.
        return Some(text.len());
    }

    None
}

/// Find pattern boundary that is the START of a consecutive run.
///
/// Searches backward from `target_end`, then forward if `forward_fallback` is true.
/// When `consecutive` is true, returns position of a pattern that is NOT preceded
/// by another instance of the same pattern.
///
/// Returns positions > start (never returns start itself, as that wouldn't make progress).
pub fn find_pattern_boundary(
    text: &[u8],
    pattern: &[u8],
    start: usize,
    target_end: usize,
    consecutive: bool,
    forward_fallback: bool,
) -> Option<usize> {
    let plen = pattern.len();
    if plen == 0 || start >= text.len() {
        return None;
    }

    let target_end = target_end.min(text.len());
    let window = &text[start..target_end];

    // Backward search
    if consecutive {
        // Find last pattern that is START of consecutive run
        let mut search_end = window.len();
        while search_end > 0 {
            let rel_pos = if plen == 1 {
                memchr::memrchr(pattern[0], &window[..search_end])
            } else {
                memmem::rfind(&window[..search_end], pattern)
            };

            if let Some(rel_pos) = rel_pos {
                let abs_pos = start + rel_pos;
                // Check if this is START of consecutive run (not preceded by same pattern)
                if abs_pos < plen || &text[abs_pos - plen..abs_pos] != pattern {
                    // Found valid boundary, but skip if it equals start (no progress)
                    if abs_pos > start {
                        return Some(abs_pos);
                    }
                    // We've traced back to start - chunk is full of consecutive patterns
                    // Fall through to forward fallback
                    break;
                }
                // In middle of run, search earlier
                search_end = rel_pos;
            } else {
                break;
            }
        }
    } else {
        // Simple case: just find last occurrence (but not at start position)
        let rel_pos = if plen == 1 {
            memchr::memrchr(pattern[0], window)
        } else {
            memmem::rfind(window, pattern)
        };
        if let Some(rel_pos) = rel_pos {
            let abs_pos = start + rel_pos;
            if abs_pos > start {
                return Some(abs_pos);
            }
        }
    }

    // Forward fallback search - find next boundary after target_end
    if forward_fallback {
        // First, determine where to start searching forward.
        // If we're at the start of a consecutive run, skip past the entire run.
        let mut forward_from = target_end;

        if consecutive && start + plen <= text.len() && &text[start..start + plen] == pattern {
            // We're at a pattern at `start`. Skip all consecutive patterns.
            let mut pos = start;
            while pos + plen <= text.len() && &text[pos..pos + plen] == pattern {
                pos += plen;
            }
            // Start searching from end of consecutive run, but not before target_end
            forward_from = forward_from.max(pos);
        }

        if forward_from < text.len() {
            let forward_window = &text[forward_from..];

            if consecutive {
                // Find first pattern that is START of consecutive run
                let mut search_start = 0;
                while search_start < forward_window.len() {
                    let rel_pos = if plen == 1 {
                        memchr::memchr(pattern[0], &forward_window[search_start..])
                    } else {
                        memmem::find(&forward_window[search_start..], pattern)
                    };

                    if let Some(rel_pos) = rel_pos {
                        let abs_pos = forward_from + search_start + rel_pos;
                        if abs_pos < plen || &text[abs_pos - plen..abs_pos] != pattern {
                            return Some(abs_pos);
                        }
                        // In middle of run, search later
                        search_start += rel_pos + 1;
                    } else {
                        break;
                    }
                }
            } else {
                let rel_pos = if plen == 1 {
                    memchr::memchr(pattern[0], forward_window)
                } else {
                    memmem::find(forward_window, pattern)
                };
                if let Some(rel_pos) = rel_pos {
                    return Some(forward_from + rel_pos);
                }
            }
        }

        // No pattern found forward. Return text.len() to include all remaining
        // text in one chunk. This avoids O(n²) behavior from repeatedly searching
        // forward through the entire remaining text on each iteration.
        return Some(text.len());
    }

    None
}

/// Compute the split position given the current state.
///
/// Returns the position to split at, handling pattern mode vs delimiter mode,
/// prefix vs suffix mode, and the special `text.len()` signal.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn compute_split_at(
    text: &[u8],
    pos: usize,
    end: usize,
    pattern: Option<&[u8]>,
    delimiters: &[u8],
    table: Option<&[bool; 256]>,
    prefix_mode: bool,
    consecutive: bool,
    forward_fallback: bool,
) -> usize {
    if let Some(pattern) = pattern {
        // Multi-byte pattern mode
        match find_pattern_boundary(text, pattern, pos, end, consecutive, forward_fallback) {
            Some(found_pos) => {
                if found_pos == text.len() {
                    // Special case: text.len() means "take all remaining"
                    found_pos
                } else if prefix_mode {
                    // Split BEFORE pattern (pattern goes to next chunk)
                    if found_pos == pos { end } else { found_pos }
                } else {
                    // Split AFTER pattern (pattern stays with current chunk)
                    found_pos + pattern.len()
                }
            }
            None => end, // No pattern found, hard split at target
        }
    } else {
        // Single-byte delimiters mode
        match find_delimiter_boundary(
            text,
            delimiters,
            table,
            pos,
            end,
            consecutive,
            forward_fallback,
        ) {
            Some(found_pos) => {
                if found_pos == text.len() {
                    // Special case: text.len() means "take all remaining"
                    found_pos
                } else if prefix_mode {
                    // Split BEFORE delimiter (delimiter goes to next chunk)
                    if found_pos == pos { end } else { found_pos }
                } else {
                    // Split AFTER delimiter (delimiter stays with current chunk)
                    found_pos + 1
                }
            }
            None => end, // No delimiter found, hard split at target
        }
    }
}
