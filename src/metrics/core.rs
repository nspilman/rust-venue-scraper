//! Core metrics abstractions and utilities
//!
//! This module provides the foundational types and utilities for metrics collection,
//! including timing helpers and common measurement patterns.

use std::time::Instant;

/// A timing guard that automatically records duration when dropped
///
/// This provides a RAII-style timing measurement that's hard to misuse.
/// The duration is automatically recorded to the specified histogram when
/// the guard goes out of scope.
pub struct TimingGuard {
    start: Instant,
    histogram_name: &'static str,
    labels: Vec<(&'static str, String)>,
}

impl TimingGuard {
    pub fn new(histogram_name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            histogram_name,
            labels: Vec::new(),
        }
    }

    pub fn with_label(mut self, key: &'static str, value: String) -> Self {
        self.labels.push((key, value));
        self
    }

    /// Manually finish the timing and record the duration
    ///
    /// This consumes the guard and records the duration. If not called,
    /// the duration will be recorded when the guard is dropped.
    pub fn finish(self) {
        // The Drop implementation will handle the recording
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        // For now, just record without labels to avoid lifetime issues
        ::metrics::histogram!(self.histogram_name).record(duration);
    }
}

/// Convenience function to create a timing guard
///
/// Usage:
/// ```rust
/// let _timing = time_operation("sms_gateway_processing_duration_seconds");
/// // ... do work ...
/// // Duration is automatically recorded when _timing goes out of scope
/// ```
pub fn time_operation(histogram_name: &'static str) -> TimingGuard {
    TimingGuard::new(histogram_name)
}

/// Convenience function to create a timing guard with labels
pub fn time_operation_with_labels(
    histogram_name: &'static str,
    labels: Vec<(&'static str, String)>,
) -> TimingGuard {
    let mut guard = TimingGuard::new(histogram_name);
    for (key, value) in labels {
        guard = guard.with_label(key, value);
    }
    guard
}

// Note: For simplicity, we'll use the metrics macros directly in the phase modules
// rather than trying to wrap them in helper functions that have lifetime issues.

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timing_guard_basic() {
        let _guard = time_operation("test_metric");
        thread::sleep(Duration::from_millis(10));
        // Duration should be recorded when guard drops
    }

    #[test]
    fn test_timing_guard_with_labels() {
        let _guard =
            time_operation_with_labels("test_metric_labeled", vec![("source", "test".to_string())]);
        thread::sleep(Duration::from_millis(10));
        // Duration should be recorded when guard drops
    }
}
