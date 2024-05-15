// Temporary workaround until the `async_event_loom` flag can be whitelisted
// without a `build.rs` [1].
//
// [1]: (https://github.com/rust-lang/rust/issues/124800).
#![allow(unexpected_cfgs)]

/// Non-Loom tests that may not leak memory; on MIRI, enabled only if
/// `tachyonix_ignore_leaks` is not configured.
#[cfg(all(not(tachyonix_loom), any(not(miri), not(tachyonix_ignore_leaks))))]
mod general;
/// Non-Loom tests that may leak memory; on MIRI, enabled only if
/// `tachyonix_ignore_leaks` is configured.
#[cfg(all(not(tachyonix_loom), any(not(miri), tachyonix_ignore_leaks)))]
mod may_leak;
