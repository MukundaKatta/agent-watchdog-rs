//! `agent-watchdog`: detect stuck or stalled LLM agent loops.
//!
//! Long-running LLM agents can get caught in unproductive loops — calling the
//! same tool over and over, repeating the same action, or otherwise failing to
//! make progress. This crate provides a tiny, dependency-free [`Watchdog`] that
//! watches a stream of "progress" events and signals when the same event has
//! repeated `threshold` times in a row.
//!
//! # How it works
//!
//! You feed the watchdog a short string describing each step the agent takes
//! (for example a tool name or an action id) via [`Watchdog::tick_progress`].
//! When the most recent `threshold` events are all identical, the watchdog
//! considers the agent *stalled*. As soon as a different event arrives, the
//! stall clears automatically.
//!
//! # Quick start
//!
//! ```rust
//! use agent_watchdog::Watchdog;
//!
//! // Trigger a stall after 5 consecutive identical events.
//! let mut wd = Watchdog::new(5);
//!
//! wd.tick_progress("search_tool"); // progress
//! wd.tick_progress("search_tool"); // same tool again
//! assert!(!wd.is_stalled()); // only 2 repeats, threshold is 5
//! ```
//!
//! # Reacting to a stall
//!
//! ```rust
//! use agent_watchdog::Watchdog;
//!
//! let mut wd = Watchdog::new(3);
//! for _ in 0..3 {
//! wd.tick_progress("call_api");
//! }
//!
//! if let Err(stall) = wd.check() {
//! // e.g. abort the loop, switch strategy, or escalate to a human.
//! assert_eq!(stall.repeated_event, "call_api");
//! eprintln!("watchdog tripped: {stall}");
//! }
//! ```

use std::collections::VecDeque;
use std::fmt;

/// Error returned when the watchdog detects a stall.
///
/// Returned by [`Watchdog::check`]. It implements [`std::error::Error`] and
/// [`Display`](std::fmt::Display) so it can be propagated with `?` or logged
/// directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StallDetected {
    /// The event string that kept repeating.
    pub repeated_event: String,
    /// How many times the event currently appears in the tracked window.
    pub repeat_count: usize,
    /// The threshold that was configured on the watchdog.
    pub threshold: usize,
}

impl fmt::Display for StallDetected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "stall detected: '{}' repeated {} times (threshold: {})",
            self.repeated_event, self.repeat_count, self.threshold
        )
    }
}

impl std::error::Error for StallDetected {}

/// Tracks recent events and reports a stall when the same event repeats
/// `threshold` times consecutively.
///
/// The watchdog keeps a bounded window of the most recent events, so its
/// memory use stays constant regardless of how long the agent runs.
#[derive(Debug, Clone)]
pub struct Watchdog {
    threshold: usize,
    recent: VecDeque<String>,
    stall_event: Option<String>,
}

impl Watchdog {
    /// Create a watchdog that triggers after `threshold` consecutive identical
    /// events.
    ///
    /// A `threshold` of `0` is meaningless (a watchdog must see at least one
    /// event before it can decide anything), so it is clamped to `1`. With a
    /// threshold of `1`, any single repeated-looking event trips the watchdog
    /// immediately.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use agent_watchdog::Watchdog;
    /// let wd = Watchdog::new(3);
    /// assert_eq!(wd.threshold(), 3);
    ///
    /// // 0 is clamped up to 1.
    /// let wd = Watchdog::new(0);
    /// assert_eq!(wd.threshold(), 1);
    /// ```
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold: threshold.max(1),
            recent: VecDeque::new(),
            stall_event: None,
        }
    }

    /// Record a progress event (e.g. a tool name or action id).
    ///
    /// Identical consecutive events count toward a stall; a different event
    /// resets the streak.
    pub fn tick_progress(&mut self, event: &str) {
        self.recent.push_back(event.to_string());
        // Keep only the last `threshold + 1` events so memory stays bounded
        // while still letting us tell a fresh stall from a continuing one.
        while self.recent.len() > self.threshold + 1 {
            self.recent.pop_front();
        }
        self.update_stall();
    }

    fn update_stall(&mut self) {
        if self.recent.len() < self.threshold {
            self.stall_event = None;
            return;
        }
        let last = match self.recent.back() {
            Some(last) => last.clone(),
            None => {
                self.stall_event = None;
                return;
            }
        };
        let consecutive = self
            .recent
            .iter()
            .rev()
            .take(self.threshold)
            .filter(|e| *e == &last)
            .count();
        if consecutive >= self.threshold {
            self.stall_event = Some(last);
        } else {
            self.stall_event = None;
        }
    }

    /// Returns `true` if a stall is currently detected.
    pub fn is_stalled(&self) -> bool {
        self.stall_event.is_some()
    }

    /// Returns `Err(StallDetected)` if the watchdog is currently stalled, or
    /// `Ok(())` otherwise.
    ///
    /// This is convenient inside an agent loop, where you can use `?` to bail
    /// out of an unproductive run.
    pub fn check(&self) -> Result<(), StallDetected> {
        if let Some(event) = &self.stall_event {
            let count = self.recent.iter().filter(|e| *e == event).count();
            Err(StallDetected {
                repeated_event: event.clone(),
                repeat_count: count,
                threshold: self.threshold,
            })
        } else {
            Ok(())
        }
    }

    /// Clear all tracked events and any detected stall, returning the watchdog
    /// to its freshly-constructed state (the threshold is preserved).
    pub fn reset(&mut self) {
        self.recent.clear();
        self.stall_event = None;
    }

    /// The configured stall threshold (always at least `1`).
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// The number of events currently held in the tracking window.
    ///
    /// This is bounded by `threshold + 1`; it is not a count of every event
    /// ever observed.
    pub fn event_count(&self) -> usize {
        self.recent.len()
    }

    /// Returns `true` if no events have been recorded since construction or the
    /// last [`reset`](Watchdog::reset).
    pub fn is_empty(&self) -> bool {
        self.recent.is_empty()
    }

    /// The most recently recorded event, or `None` if none have been recorded.
    pub fn last_event(&self) -> Option<&str> {
        self.recent.back().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_stall_under_threshold() {
        let mut wd = Watchdog::new(3);
        wd.tick_progress("a");
        wd.tick_progress("a");
        assert!(!wd.is_stalled());
    }

    #[test]
    fn stall_at_threshold() {
        let mut wd = Watchdog::new(3);
        wd.tick_progress("a");
        wd.tick_progress("a");
        wd.tick_progress("a");
        assert!(wd.is_stalled());
    }

    #[test]
    fn different_events_no_stall() {
        let mut wd = Watchdog::new(3);
        wd.tick_progress("a");
        wd.tick_progress("b");
        wd.tick_progress("a");
        assert!(!wd.is_stalled());
    }

    #[test]
    fn check_ok_when_not_stalled() {
        let mut wd = Watchdog::new(3);
        wd.tick_progress("a");
        assert!(wd.check().is_ok());
    }

    #[test]
    fn check_err_when_stalled() {
        let mut wd = Watchdog::new(2);
        wd.tick_progress("x");
        wd.tick_progress("x");
        assert!(wd.check().is_err());
    }

    #[test]
    fn error_has_event_name() {
        let mut wd = Watchdog::new(2);
        wd.tick_progress("loop_tool");
        wd.tick_progress("loop_tool");
        let err = wd.check().unwrap_err();
        assert_eq!(err.repeated_event, "loop_tool");
        assert!(err.to_string().contains("loop_tool"));
    }

    #[test]
    fn reset_clears_stall() {
        let mut wd = Watchdog::new(2);
        wd.tick_progress("x");
        wd.tick_progress("x");
        wd.reset();
        assert!(!wd.is_stalled());
        assert_eq!(wd.event_count(), 0);
    }

    #[test]
    fn recovery_after_stall() {
        let mut wd = Watchdog::new(3);
        wd.tick_progress("a");
        wd.tick_progress("a");
        wd.tick_progress("a");
        assert!(wd.is_stalled());
        wd.tick_progress("b");
        assert!(!wd.is_stalled());
    }

    #[test]
    fn last_event() {
        let mut wd = Watchdog::new(5);
        wd.tick_progress("first");
        wd.tick_progress("last");
        assert_eq!(wd.last_event(), Some("last"));
    }

    #[test]
    fn last_event_empty() {
        let wd = Watchdog::new(3);
        assert_eq!(wd.last_event(), None);
    }

    #[test]
    fn threshold_getter() {
        let wd = Watchdog::new(7);
        assert_eq!(wd.threshold(), 7);
    }

    #[test]
    fn stall_then_progress_then_stall_again() {
        let mut wd = Watchdog::new(2);
        wd.tick_progress("a");
        wd.tick_progress("a"); // stall
        assert!(wd.is_stalled());
        wd.tick_progress("b"); // recovery
        assert!(!wd.is_stalled());
        wd.tick_progress("b"); // stall again
        assert!(wd.is_stalled());
    }

    #[test]
    fn threshold_one_stalls_immediately() {
        let mut wd = Watchdog::new(1);
        wd.tick_progress("x");
        assert!(wd.is_stalled());
    }

    #[test]
    fn threshold_zero_is_clamped_to_one() {
        let mut wd = Watchdog::new(0);
        assert_eq!(wd.threshold(), 1);
        // With no events it must not report a stall...
        assert!(!wd.is_stalled());
        // ...but a single event trips a threshold-1 watchdog.
        wd.tick_progress("x");
        assert!(wd.is_stalled());
    }

    #[test]
    fn is_empty_tracks_window() {
        let mut wd = Watchdog::new(2);
        assert!(wd.is_empty());
        wd.tick_progress("a");
        assert!(!wd.is_empty());
        wd.reset();
        assert!(wd.is_empty());
    }

    #[test]
    fn window_stays_bounded() {
        let mut wd = Watchdog::new(3);
        for _ in 0..100 {
            wd.tick_progress("a");
        }
        // Window is capped at threshold + 1 regardless of how many ticks happen.
        assert_eq!(wd.event_count(), 4);
        assert!(wd.is_stalled());
    }

    #[test]
    fn error_is_cloneable_and_comparable() {
        let mut wd = Watchdog::new(2);
        wd.tick_progress("z");
        wd.tick_progress("z");
        let a = wd.check().unwrap_err();
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn watchdog_is_cloneable() {
        let mut wd = Watchdog::new(2);
        wd.tick_progress("a");
        wd.tick_progress("a");
        let copy = wd.clone();
        assert!(copy.is_stalled());
        assert_eq!(copy.last_event(), Some("a"));
    }
}
