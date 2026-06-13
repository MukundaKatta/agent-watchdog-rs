# agent-watchdog

A tiny, dependency-free Rust library that detects **stuck or stalled LLM agent loops**.

When an autonomous agent gets caught in a loop — calling the same tool or
repeating the same action over and over without making progress — `agent-watchdog`
notices and lets you break out before you burn tokens, time, or money.

## How it works

You feed the watchdog a stream of *progress events* (a tool name, an action id,
a step label — anything that should change as the agent makes real progress).
The watchdog keeps a small sliding window of the most recent events. When the
same event repeats `threshold` times in a row, it flags a stall.

## Installation

Add it to your `Cargo.toml`:

```toml
[dependencies]
agent-watchdog = "0.1"
```

## Usage

```rust
use agent_watchdog::Watchdog;

// Trigger a stall after 3 consecutive identical events.
let mut wd = Watchdog::new(3);

wd.tick_progress("search_tool");   // event 1
wd.tick_progress("search_tool");   // event 2
wd.tick_progress("search_tool");   // event 3 -> stalled

if wd.is_stalled() {
    // The agent is looping; intervene (abort, re-plan, escalate, ...).
    if let Err(stall) = wd.check() {
        eprintln!("{stall}");
        // -> stall detected: 'search_tool' repeated 3 times (threshold: 3)
    }
}

// A different event counts as progress and clears the stall.
wd.tick_progress("write_tool");
assert!(!wd.is_stalled());
```

## API overview

| Method | Description |
| ------ | ----------- |
| `Watchdog::new(threshold)` | Create a watchdog that stalls after `threshold` consecutive identical events. |
| `tick_progress(event)` | Record a progress event. |
| `is_stalled()` | `true` if a stall is currently detected. |
| `check()` | Returns `Err(StallDetected)` with details when stalled, otherwise `Ok(())`. |
| `reset()` | Clear all tracked state. |
| `last_event()` | The most recent event, if any. |
| `event_count()` | Number of events currently in the window. |
| `threshold()` | The configured stall threshold. |

`StallDetected` implements `std::error::Error` and `Display`, so it composes
cleanly with `?` and standard error-handling.

## Building and testing

```sh
cargo build
cargo test
```

## Tech stack

- **Language:** Rust (edition 2021)
- **Dependencies:** none — pure `std` (`VecDeque`)

## License

Licensed under the [MIT License](Cargo.toml).
