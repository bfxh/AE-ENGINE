//! 性能追踪模块（借鉴 rend3 + profiling crate）

pub mod profiler;

pub use profiler::{Profiler, ProfileSpan, SpanId};