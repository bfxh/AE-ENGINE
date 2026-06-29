//! Visit trait（借鉴 Fyrox）
//!
//! 类似 serde 但更轻量，支持反向访问（读/写双向）

#[derive(Debug, Clone)]
pub enum VisitError {
    Io(String),
    TypeMismatch,
    UnknownField(String),
    InvalidData(String),
}

impl std::fmt::Display for VisitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) => write!(f, "io: {}", s),
            Self::TypeMismatch => write!(f, "type mismatch"),
            Self::UnknownField(s) => write!(f, "unknown field: {}", s),
            Self::InvalidData(s) => write!(f, "invalid data: {}", s),
        }
    }
}
impl std::error::Error for VisitError {}

/// Visit 上下文（读 or 写）
pub enum VisitContext<'a> {
    Read { data: &'a [u8], pos: usize },
    Write { data: &'a mut Vec<u8> },
}

impl<'a> VisitContext<'a> {
    pub fn new_read(data: &'a [u8]) -> Self {
        Self::Read { data, pos: 0 }
    }
    pub fn new_write(data: &'a mut Vec<u8>) -> Self {
        Self::Write { data }
    }
    pub fn is_reading(&self) -> bool {
        matches!(self, Self::Read { .. })
    }
}

/// Visit trait
pub trait Visit: Sized {
    fn visit(&mut self, ctx: &mut VisitContext) -> Result<(), VisitError>;
}

/// Visitor
pub struct Visitor;

impl Visitor {
    pub fn visit_value<T: Visit>(ctx: &mut VisitContext, value: &mut T) -> Result<(), VisitError> {
        value.visit(ctx)
    }
}

macro_rules! impl_visit_primitive {
    ($($t:ty),*) => {
        $(
            impl Visit for $t {
                fn visit(&mut self, ctx: &mut VisitContext) -> Result<(), VisitError> {
                    match ctx {
                        VisitContext::Read { data, pos } => {
                            if *pos + std::mem::size_of::<$t>() > data.len() {
                                return Err(VisitError::Io("eof".into()));
                            }
                            let bytes = &data[*pos..*pos + std::mem::size_of::<$t>()];
                            *self = <$t>::from_le_bytes(bytes.try_into().unwrap());
                            *pos += std::mem::size_of::<$t>();
                        }
                        VisitContext::Write { data } => {
                            let bytes = self.to_le_bytes();
                            data.extend_from_slice(&bytes);
                        }
                    }
                    Ok(())
                }
            }
        )*
    };
}

impl_visit_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);