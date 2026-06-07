# agent-watchdog

[![CI](https://github.com/MukundaKatta/agent-watchdog-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/MukundaKatta/agent-watchdog-rs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

A tiny, dependency-free Rust crate that detects when an LLM agent gets stuck in
an unproductive loop.

Long-running agents sometimes spin their wheels — calling the same tool over and
over, repeating the same action, or otherwise failing to make progress.
`agent-watchdog` watches a stream of "progress" events and signals as soon as the
same event has repeated a configurable number of times in a row, so your loop can
abort, switch strategy, or escalate to a human instead of burning tokens.

## Features

- **Zero dependencies** — pure `std`, compiles instantly.
- **Constant memory** — keeps only a small bounded window of recent events,
  regardless of how long the agent runs.
- **Self-clearing** — a stall resolves automatically as soon as a different
  event arrives.
- **Ergonomic** — returns a real `Error` you can propagate with `?`.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
agent-watchdog = "0.1"
```

Or with cargo:

```sh
cargo add agent-watchdog
```

## Usage

Feed the watchdog one short string per agent step (a tool name, an action id,
etc.). When the last `threshold` events are identical, it reports a stall.

```rust
use agent_watchdog::Watchdog;

// Consider the agent stalled after 3 identical consecutive steps.
let mut wd = Watchdog::new(3);

// A healthy run never trips the watchdog.
for tool in ["plan", "search", "read", "search", "edit"] {
    wd.tick_progress(tool);
    assert!(wd.check().is_ok());
}

// But repeating the same tool does.
wd.tick_progress("search");
wd.tick_progress("search");
wd.tick_progress("search");

if let Err(stall) = wd.check() {
    eprintln!("watchdog tripped: {stall}");
    assert_eq!(stall.repeated_event, "search");
}
```

A runnable version of this pattern lives in
[`examples/loop_guard.rs`](examples/loop_guard.rs):

```sh
cargo run --example loop_guard
```

## API

### `Watchdog`

| Method | Description |
| --- | --- |
| `Watchdog::new(threshold)` | Create a watchdog that trips after `threshold` consecutive identical events. A `threshold` of `0` is clamped to `1`. |
| `tick_progress(&mut self, event: &str)` | Record one progress event. |
| `check(&self) -> Result<(), StallDetected>` | `Err` if currently stalled, otherwise `Ok`. Convenient with `?`. |
| `is_stalled(&self) -> bool` | Whether a stall is currently detected. |
| `reset(&mut self)` | Clear all tracked events and any stall (threshold preserved). |
| `threshold(&self) -> usize` | The configured threshold (always `>= 1`). |
| `event_count(&self) -> usize` | Number of events in the bounded window (`<= threshold + 1`). |
| `is_empty(&self) -> bool` | `true` if no events have been recorded since construction/reset. |
| `last_event(&self) -> Option<&str>` | The most recently recorded event. |

### `StallDetected`

The error returned by `check`. It implements `Display`, `std::error::Error`,
`Clone`, and `PartialEq`, and exposes:

- `repeated_event: String` — the event that kept repeating.
- `repeat_count: usize` — how many times it appears in the tracked window.
- `threshold: usize` — the threshold that was configured.

## How stall detection works

Each call to `tick_progress` appends the event to a window holding at most
`threshold + 1` items. A stall is flagged when the most recent `threshold` events
are all identical. Because the window is bounded, memory use stays constant no
matter how long the agent runs, and the stall flag clears automatically the
moment a different event is observed.

## Development

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## License

Licensed under the [MIT License](LICENSE).
