use criterion::{Criterion, black_box, criterion_group, criterion_main};
use wasteland_serialize::binary::{BinaryReader, BinaryWriter};
use wasteland_serialize::bitpack::BitPacker;
use wasteland_serialize::soa::SoA;
use wasteland_serialize::zerocopy::ZeroCopyBuf;

fn bench_binary_write_read(c: &mut Criterion) {
    c.bench_function("serialize_binary_write_1000", |bench| {
        bench.iter(|| {
            let mut writer = BinaryWriter::new(8000);
            for i in 0..1000u32 {
                writer.write_u32(black_box(i));
                writer.write_f32(black_box(i as f32 * 0.5));
            }
            black_box(writer.into_vec());
        });
    });

    c.bench_function("serialize_binary_read_1000", |bench| {
        let mut writer = BinaryWriter::new(8000);
        for i in 0..1000u32 {
            writer.write_u32(i);
            writer.write_f32(i as f32 * 0.5);
        }
        let writer_data = writer.into_vec();

        bench.iter(|| {
            let mut reader = BinaryReader::new(&writer_data);
            for _i in 0..1000u32 {
                let v = reader.read_u32().unwrap();
                let f = reader.read_f32().unwrap();
                black_box((v, f));
            }
        });
    });
}

fn bench_bitpack(c: &mut Criterion) {
    c.bench_function("serialize_bitpack_1000", |bench| {
        bench.iter(|| {
            let mut packer = BitPacker::new();
            for i in 0..1000u32 {
                packer.write_bits(black_box(i as u64), 10);
            }
            black_box(packer.bit_count());
        });
    });
}

fn bench_soa_push(c: &mut Criterion) {
    c.bench_function("serialize_soa_push_10000", |bench| {
        bench.iter(|| {
            let mut store = SoA::<f32, 3>::new();
            for i in 0..10000 {
                store.push(black_box([(i as f32) * 0.1, (i as f32) * 0.2, (i as f32) * 0.3]));
            }
            black_box(store.len());
        });
    });
}

fn bench_zerocopy(c: &mut Criterion) {
    let src: Vec<u8> = (0..10000u32).flat_map(|i| i.to_le_bytes()).collect();

    c.bench_function("serialize_zerocopy_read_10000", |bench| {
        bench.iter(|| {
            let mut buf = ZeroCopyBuf::new(src.len());
            for (i, &b) in src.iter().enumerate() {
                buf.write(i, b);
            }
            let mut offset = 0;
            for _ in 0..2500u32 {
                let v = buf.read::<u32>(offset).unwrap();
                offset += 4;
                black_box(v);
            }
        });
    });
}

criterion_group!(benches, bench_binary_write_read, bench_bitpack, bench_soa_push, bench_zerocopy);
criterion_main!(benches);
