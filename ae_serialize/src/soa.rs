pub struct SoA<T, const N: usize> {
    fields: Vec<[T; N]>,
    len: usize,
}

impl<T: Copy + Default, const N: usize> SoA<T, N> {
    pub fn new() -> Self {
        SoA { fields: Vec::new(), len: 0 }
    }

    pub fn with_capacity(cap: usize) -> Self {
        SoA { fields: Vec::with_capacity(cap), len: 0 }
    }

    pub fn push(&mut self, values: [T; N]) {
        self.fields.push(values);
        self.len += 1;
    }

    pub fn get(&self, index: usize) -> Option<&[T; N]> {
        self.fields.get(index)
    }

    pub fn get_field(&self, index: usize, field: usize) -> Option<&T> {
        self.fields.get(index).and_then(|row| row.get(field))
    }

    pub fn column(&self, field: usize) -> ColumnIter<'_, T, N> {
        ColumnIter { soa: self, field, index: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &[T; N]> {
        self.fields.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut [T; N]> {
        self.fields.iter_mut()
    }

    pub fn as_slice(&self) -> &[[T; N]] {
        &self.fields
    }

    pub fn memory_layout(&self) -> SoALayout {
        let struct_size = std::mem::size_of::<[T; N]>();
        SoALayout {
            entity_count: self.len,
            field_count: N,
            field_size: std::mem::size_of::<T>(),
            row_size: struct_size,
            total_bytes: self.fields.len() * struct_size,
            padding: struct_size - N * std::mem::size_of::<T>(),
        }
    }
}

impl<T: Copy + Default, const N: usize> Default for SoA<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ColumnIter<'a, T, const N: usize> {
    soa: &'a SoA<T, N>,
    field: usize,
    index: usize,
}

impl<'a, T: Copy + Default, const N: usize> Iterator for ColumnIter<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.soa.len {
            return None;
        }
        let value = self.soa.get(self.index)?.get(self.field)?;
        self.index += 1;
        Some(value)
    }
}

#[derive(Debug, Clone)]
pub struct SoALayout {
    pub entity_count: usize,
    pub field_count: usize,
    pub field_size: usize,
    pub row_size: usize,
    pub total_bytes: usize,
    pub padding: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_get() {
        let mut soa = SoA::<f32, 3>::new();
        soa.push([1.0, 2.0, 3.0]);
        soa.push([4.0, 5.0, 6.0]);
        assert_eq!(soa.len(), 2);
        assert_eq!(soa.get(0), Some(&[1.0, 2.0, 3.0]));
        assert_eq!(soa.get_field(1, 2), Some(&6.0));
    }

    #[test]
    fn test_column_iter() {
        let mut soa = SoA::<f32, 3>::new();
        soa.push([1.0, 2.0, 3.0]);
        soa.push([4.0, 5.0, 6.0]);
        let col0: Vec<f32> = soa.column(0).copied().collect();
        assert_eq!(col0, vec![1.0, 4.0]);
        let col1: Vec<f32> = soa.column(1).copied().collect();
        assert_eq!(col1, vec![2.0, 5.0]);
    }

    #[test]
    fn test_memory_layout() {
        let mut soa = SoA::<f32, 4>::new();
        soa.push([0.0; 4]);
        let layout = soa.memory_layout();
        assert_eq!(layout.entity_count, 1);
        assert_eq!(layout.field_count, 4);
        assert_eq!(layout.field_size, 4);
    }

    #[test]
    fn test_empty() {
        let soa = SoA::<f32, 3>::new();
        assert!(soa.is_empty());
        assert_eq!(soa.len(), 0);
        assert_eq!(soa.get(0), None);
    }
}
