//! Tracy real-time frame profiler integration (behind the `tracy` feature flag).
//!
//! When `feature = "tracy"` is OFF (the default), `span!` / `plot!` are no-op macros with
//! zero overhead and no dependency on the `tracy-client` C++/Rust binding, and `frame_mark()`
//! is an empty function. The normal build never compiles the profiler in.
//!
//! When ON, `span!` / `plot!` forward to the `tracy-client` crate and `frame_mark()` marks the
//! end of a frame for Tracy's frame-time graph.
//!
//! IMPORTANT: enabling Tracy makes the client broadcast discovery packets on your local
//! network. Only build with `--features tracy` when you are actively profiling, and never ship
//! a default-enabled Tracy build.

#[cfg(feature = "tracy")]
pub use tracy_client::frame_mark;

#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! span {
    ($name:expr) => { () };
}

#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! plot {
    ($name:expr, $val:expr) => {};
}

#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! frame_mark {
    () => {};
}

#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! frame_mark {
    () => {
        $crate::profiling::tracy_client_frame_mark()
    };
}

/// Re-export of the real Tracy frame mark (used by the `frame_mark!` macro under the feature).
#[cfg(feature = "tracy")]
pub fn tracy_client_frame_mark() {
    tracy_client::frame_mark();
}

#[cfg(not(feature = "tracy"))]
pub fn frame_mark() {}

/// Start the Tracy client capture. No-op when the feature is off.
///
/// `Client::running()` lazily spins up the capture thread that broadcasts to the Tracy GUI.
#[cfg(feature = "tracy")]
pub fn start() {
    let _ = tracy_client::Client::running();
}

#[cfg(not(feature = "tracy"))]
pub fn start() {}
