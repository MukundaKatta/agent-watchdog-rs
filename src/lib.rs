/*!
agent-watchdog: detect stuck or stalled LLM agent loops.

```rust
use agent_watchdog::Watchdog;

let mut wd = Watchdog::new(5);       // stall after 5 ticks of no progress
wd.tick_progress("tool_a");          // progress: tool changed
wd.tick_progress("tool_a");          // no change in tool name
assert!(!wd.is_stalled());           // only 1 repeat
```
*/

use std::collections::VecDeque;
use std::fmt;

/// Raised when the watchdog detects a stall.
#[derive(Debug)]
pub struct StallDetected {
    pub repeated_event: String,
    pub repeat_count: usize,
    pub threshold: usize,
}

impl fmt::Display for StallDetected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stall detected: '{}' repeated {} times (threshold: {})",
            self.repeated_event, self.repeat_count, self.threshold)
    }
}

impl std::error::Error for StallDetected {}

/// Tracks recent events and raises when the same event repeats too many times.
pub struct Watchdog {
    threshold: usize,
    recent: VecDeque<String>,
    stall_event: Option<String>,
}

impl Watchdog {
    /// Create a watchdog that triggers after `threshold` consecutive identical events.
    pub fn new(threshold: usize) -> Self {
        Self { threshold, recent: VecDeque::new(), stall_event: None }
    }

    /// Record a progress event (e.g. tool name, action id).
    pub fn tick_progress(&mut self, event: &str) {
        self.recent.push_back(event.to_string());
        // Keep only last `threshold + 1` events.
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
        let last = self.recent.back().cloned().unwrap_or_default();
        let consecutive = self.recent.iter().rev().take(self.threshold).filter(|e| *e == &last).count();
        if consecutive >= self.threshold {
            self.stall_event = Some(last);
        } else {
            self.stall_event = None;
        }
    }

    /// True if a stall has been detected.
    pub fn is_stalled(&self) -> bool { self.stall_event.is_some() }

    /// Return Err(StallDetected) if stalled.
    pub fn check(&self) -> Result<(), StallDetected> {
        if let Some(event) = &self.stall_event {
            let count = self.recent.iter().filter(|e| *e == event).count();
            Err(StallDetected { repeated_event: event.clone(), repeat_count: count, threshold: self.threshold })
        } else {
            Ok(())
        }
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        self.recent.clear();
        self.stall_event = None;
    }

    pub fn threshold(&self) -> usize { self.threshold }
    pub fn event_count(&self) -> usize { self.recent.len() }

    /// Most recent event.
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
        wd.tick_progress("a"); wd.tick_progress("a"); // stall
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
}
