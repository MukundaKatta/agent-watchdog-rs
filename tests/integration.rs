//! Integration tests exercising the public API of `agent-watchdog` exactly as
//! a downstream crate would use it.

use agent_watchdog::{StallDetected, Watchdog};

#[test]
fn detects_a_tool_loop_in_a_realistic_sequence() {
    let mut wd = Watchdog::new(3);

    // A healthy run alternates between tools and never trips.
    for tool in ["plan", "search", "read", "search", "edit"] {
        wd.tick_progress(tool);
        assert!(wd.check().is_ok(), "no stall expected during healthy run");
    }

    // Then the agent gets stuck calling the same tool.
    wd.tick_progress("search");
    wd.tick_progress("search");
    wd.tick_progress("search");

    let err = wd.check().expect_err("should be stalled after 3 repeats");
    assert_eq!(err.repeated_event, "search");
    assert_eq!(err.threshold, 3);
}

#[test]
fn stall_clears_when_progress_resumes() {
    let mut wd = Watchdog::new(2);
    wd.tick_progress("a");
    wd.tick_progress("a");
    assert!(wd.is_stalled());

    wd.tick_progress("b");
    assert!(!wd.is_stalled());
    assert!(wd.check().is_ok());
}

#[test]
fn error_propagates_with_question_mark() {
    fn run_step(wd: &mut Watchdog, event: &str) -> Result<(), StallDetected> {
        wd.tick_progress(event);
        wd.check()?;
        Ok(())
    }

    let mut wd = Watchdog::new(2);
    assert!(run_step(&mut wd, "step").is_ok());
    let result = run_step(&mut wd, "step");
    assert!(result.is_err());
}

#[test]
fn reset_allows_reuse_across_runs() {
    let mut wd = Watchdog::new(2);
    wd.tick_progress("x");
    wd.tick_progress("x");
    assert!(wd.is_stalled());

    wd.reset();
    assert!(wd.is_empty());
    assert!(!wd.is_stalled());
    assert_eq!(wd.last_event(), None);

    // The same instance keeps working after a reset.
    wd.tick_progress("y");
    assert!(!wd.is_stalled());
}

#[test]
fn display_message_is_human_readable() {
    let mut wd = Watchdog::new(2);
    wd.tick_progress("retry");
    wd.tick_progress("retry");
    let err = wd.check().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("retry"));
    assert!(msg.contains("threshold"));
}
