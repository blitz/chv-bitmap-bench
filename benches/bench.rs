use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::{Rng, SeedableRng};

use chv_bitmap_bench::{MemoryRangeTable, bitmap_to_memory_table, bitmap_to_memory_table_opt2};

fn random_bits_vector(size: usize, bits: usize) -> Vec<u64> {
    let mut vec = vec![0; size];
    let mut rng = rand::rngs::SmallRng::seed_from_u64(12345);

    for _ in 0..bits {
        let pos = rng.random_range(0..vec.len());
        vec[pos] |= 1 << (rng.random_range(0..64));
    }

    vec
}

fn criterion_benchmark(c: &mut Criterion) {
    const PAGE_SIZE: usize = 4096;
    const VM_SIZE: usize = 12 /* TiB */ << 40;
    const DIRTY_VECTOR_LENGTH: usize = VM_SIZE / (PAGE_SIZE * 64);

    eprintln!(
        "{} MiB dirty vector for {} TiB VM",
        (DIRTY_VECTOR_LENGTH * 8) >> 20,
        VM_SIZE >> 40
    );

    for dirty_permille in [
        0,  // The empty bitmap.
        1,  // An almost empty bitmap
        10, // Probably more realistic values
        50,
    ] {
        let dirty_bits = (DIRTY_VECTOR_LENGTH * (u64::BITS as usize) * dirty_permille) / 1000;

        let vec1 = black_box(random_bits_vector(DIRTY_VECTOR_LENGTH, dirty_bits));
        let vec2 = black_box(random_bits_vector(DIRTY_VECTOR_LENGTH, dirty_bits));

        c.bench_function(&format!("dirty_log {dirty_permille}"), |b| {
            b.iter(|| black_box(bitmap_to_memory_table(&vec1, &vec2)))
        });
        c.bench_function(&format!("dirty_log {dirty_permille} (optimized)"), |b| {
            b.iter(|| black_box(bitmap_to_memory_table_opt2(&vec1, &vec2)))
        });
        c.bench_function(&format!("dirty_log {dirty_permille} (async)"), |b| {
            b.iter(|| {
                MemoryRangeTable::dirty_range_iter(
                    black_box(&vec1)
                        .iter()
                        .zip(black_box(&vec2))
                        .map(|(a, b)| a | b),
                    black_box(0),
                    black_box(4096),
                )
                .for_each(|r| {
                    black_box(r);
                });
            })
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
