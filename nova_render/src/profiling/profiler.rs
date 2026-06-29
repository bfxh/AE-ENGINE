//! Profiler
//!
//! 零开销抽象：未启用 profile feature 时编译期消除

#[cfg(feature = "profile")]
pub use profiling::*;

pub struct Profiler;

impl Profiler {
    pub fn new() -> Self { Self }
    pub fn begin_span(_name: &str) -> SpanId { SpanId(0) }
    pub fn end_span(_id: SpanId) {}
}

#[derive(Debug, Clone, Copy)]
pub struct SpanId(pub u64);

pub struct ProfileSpan {
    id: SpanId,
}

impl ProfileSpan {
    pub fn new(name: &str) -> Self {
        let id = Profiler::begin_span(name);
        Self { id }
    }
}

impl Drop for ProfileSpan {
    fn drop(&mut self) {
        Profiler::end_span(self.id);
    }
}

impl Default for Profiler {
    fn default() -> Self { Self::new() }
}