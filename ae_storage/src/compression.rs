pub fn rle_encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count = 1usize;
        while i + count < data.len() && data[i + count] == byte && count < 255 {
            count += 1;
        }
        if count > 2 || byte == 0xff {
            out.push(0xff);
            out.push(byte);
            out.push(count as u8);
        } else {
            for _ in 0..count {
                out.push(byte);
            }
        }
        i += count;
    }
    out
}

pub fn rle_decode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(data.len() * 2);
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0xff && i + 2 < data.len() {
            let byte = data[i + 1];
            let count = data[i + 2] as usize;
            for _ in 0..count {
                out.push(byte);
            }
            i += 3;
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

pub fn delta_encode_u32(data: &[u32]) -> Vec<u32> {
    if data.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(data.len());
    out.push(data[0]);
    for i in 1..data.len() {
        out.push(data[i].wrapping_sub(data[i - 1]));
    }
    out
}

pub fn delta_decode_u32(data: &[u32]) -> Vec<u32> {
    if data.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(data.len());
    out.push(data[0]);
    for i in 1..data.len() {
        out.push(out[i - 1].wrapping_add(data[i]));
    }
    out
}

pub fn compress_snapshot(data: &[u8]) -> Vec<u8> {
    rle_encode(data)
}

pub fn decompress_snapshot(data: &[u8]) -> Vec<u8> {
    rle_decode(data)
}

pub fn compression_ratio(original: usize, compressed: usize) -> f32 {
    if original == 0 {
        return 1.0;
    }
    compressed as f32 / original as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_roundtrip() {
        let data = vec![0, 0, 0, 0, 0, 1, 2, 3, 3, 3];
        let encoded = rle_encode(&data);
        let decoded = rle_decode(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_rle_compress_repeated() {
        let data = vec![0u8; 1000];
        let encoded = rle_encode(&data);
        assert!(encoded.len() < 100);
        let decoded = rle_decode(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_rle_no_compress_unique() {
        let data: Vec<u8> = (0..100).map(|i| i as u8).collect();
        let encoded = rle_encode(&data);
        let decoded = rle_decode(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_delta_u32_roundtrip() {
        let data = vec![100, 105, 108, 200, 199, 0, 10];
        let encoded = delta_encode_u32(&data);
        let decoded = delta_decode_u32(&encoded);
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(rle_encode(&[]), vec![]);
        assert_eq!(rle_decode(&[]), vec![]);
        assert_eq!(delta_encode_u32(&[]), vec![]);
        assert_eq!(delta_decode_u32(&[]), vec![]);
    }

    #[test]
    fn test_compression_ratio() {
        assert_eq!(compression_ratio(0, 0), 1.0);
        assert!(compression_ratio(1000, 100) < 0.5);
        assert!(compression_ratio(100, 1000) > 1.0);
    }
}
