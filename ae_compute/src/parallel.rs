use rayon::prelude::*;

pub fn parallel_for<T, F>(data: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync + Send,
{
    data.par_iter_mut().enumerate().for_each(|(i, item)| {
        f(i, item);
    });
}

pub fn parallel_map<T, U, F>(data: &[T], f: F) -> Vec<U>
where
    T: Sync,
    U: Send,
    F: Fn(&T) -> U + Sync + Send,
{
    data.par_iter().map(f).collect()
}

pub fn parallel_reduce<T, F>(data: &[T], identity: T, f: F) -> T
where
    T: Send + Sync + Clone,
    F: Fn(T, T) -> T + Sync + Send,
{
    data.par_iter().cloned().reduce(|| identity.clone(), f)
}

pub fn parallel_fold<T, U, F, G>(data: &[T], identity: U, fold: F, reduce: G) -> U
where
    T: Sync,
    U: Send + Clone + Sync,
    F: Fn(&mut U, &T) + Sync + Send,
    G: Fn(U, U) -> U + Sync + Send,
{
    data.par_iter()
        .fold(
            || identity.clone(),
            |mut acc, item| {
                fold(&mut acc, item);
                acc
            },
        )
        .reduce(|| identity.clone(), reduce)
}

pub fn parallel_scan<T, U, F>(data: &[T], identity: U, f: F) -> Vec<U>
where
    T: Sync,
    U: Send + Clone + Sync,
    F: Fn(&U, &T) -> U + Sync + Send,
{
    if data.is_empty() {
        return Vec::new();
    }
    let num_threads = rayon::current_num_threads().max(1);
    let chunk_size = data.len().div_ceil(num_threads);
    let mut result = vec![identity.clone(); data.len()];

    let chunks: Vec<_> =
        data.par_chunks(chunk_size).zip(result.par_chunks_mut(chunk_size)).collect();

    chunks.into_par_iter().for_each(|(input_chunk, output_chunk)| {
        let mut acc = identity.clone();
        for (i, item) in input_chunk.iter().enumerate() {
            acc = f(&acc, item);
            output_chunk[i] = acc.clone();
        }
    });

    result
}

pub fn parallel_sort<T: Ord + Send>(data: &mut [T]) {
    data.par_sort();
}

pub fn parallel_sort_by<T: Send, F: Fn(&T, &T) -> std::cmp::Ordering + Sync>(
    data: &mut [T],
    compare: F,
) {
    data.par_sort_by(compare);
}

pub fn parallel_filter<T: Send, F: Fn(&T) -> bool + Sync + Send>(
    data: Vec<T>,
    predicate: F,
) -> Vec<T> {
    data.into_par_iter().filter(predicate).collect()
}

pub fn chunked_process<T, F>(data: &mut [T], chunk_size: usize, f: F)
where
    T: Send,
    F: Fn(&mut [T]) + Sync + Send,
{
    data.par_chunks_mut(chunk_size).for_each(f);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_for() {
        let mut data = vec![0u32; 1000];
        parallel_for(&mut data, |i, v| *v = i as u32);
        for (i, v) in data.iter().enumerate() {
            assert_eq!(*v, i as u32);
        }
    }

    #[test]
    fn test_parallel_map() {
        let data = vec![1, 2, 3, 4, 5];
        let result = parallel_map(&data, |v| v * 2);
        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_parallel_reduce() {
        let data = vec![1, 2, 3, 4, 5];
        let sum = parallel_reduce(&data, 0, |a, b| a + b);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_parallel_sort() {
        let mut data = vec![5, 3, 1, 4, 2];
        parallel_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_parallel_filter() {
        let data = vec![1, 2, 3, 4, 5, 6];
        let result = parallel_filter(data, |v| v % 2 == 0);
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_parallel_fold() {
        let data = vec![1, 2, 3, 4, 5];
        let result = parallel_fold(
            &data,
            (0, 0),
            |acc: &mut (i32, i32), v: &i32| {
                acc.0 += v;
                acc.1 += 1;
            },
            |a, b| (a.0 + b.0, a.1 + b.1),
        );
        assert_eq!(result, (15, 5));
    }

    #[test]
    fn test_chunked_process() {
        let mut data = vec![0u32; 200];
        chunked_process(&mut data, 50, |chunk| {
            for item in chunk {
                *item += 1;
            }
        });
        assert!(data.iter().all(|v| *v == 1));
    }
}
