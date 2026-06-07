//! A runnable example showing how to guard an agent loop with the watchdog.
//!
//! Run with:
//!
//! ```text
//! cargo run --example loop_guard
//! ```

use agent_watchdog::Watchdog;

/// Stand-in for "what would the agent do next?". This naive agent gets stuck
/// repeatedly calling the same tool, which is exactly what the watchdog exists
/// to catch.
fn decide_next_tool(step: usize) -> &'static str {
    match step {
        0 => "plan",
        1 => "search",
        2 => "read",
        // From here on the agent loops on the same tool forever.
        _ => "search",
    }
}

fn main() {
    // Consider the agent stalled after 4 identical consecutive steps.
    let mut wd = Watchdog::new(4);

    for step in 0..20 {
        let tool = decide_next_tool(step);
        wd.tick_progress(tool);
        println!("step {step:>2}: called `{tool}`");

        if let Err(stall) = wd.check() {
            println!("\nWatchdog tripped: {stall}");
            println!("Aborting the run after {} steps.", step + 1);
            return;
        }
    }

    println!("\nRun completed without stalling.");
}
