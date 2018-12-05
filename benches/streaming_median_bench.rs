#[macro_use]
extern crate criterion;
extern crate xorshift;
//    #![allow(unused_imports)]
//    use super::*;
extern crate streaming_median;

use streaming_median::StreamingMedian;

use criterion::Criterion;
use xorshift::{Xoroshiro128, SeedableRng};
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use xorshift::Rng;

fn bench_insert_and_calculate(c: &mut Criterion) {

    c.bench_function("insert_and_calculate", |b| {
        let mut median_tracker = StreamingMedian::new(123);

        b.iter(|| {
            median_tracker.insert_and_calculate(100);
        });
    });

}


fn bench_insert_and_calculate_rand(c: &mut Criterion) {

    c.bench_function("insert_and_calculate_rand", |b| {
        let mut rng = Xoroshiro128::from_seed(&[1, 71, 1223]);

        let mut median_tracker = StreamingMedian::new(123_000);

        b.iter(|| {
            median_tracker.insert_and_calculate(rng.gen());
        });
    });
}


fn bench_insert_and_calculate_rand_within_bound(c: &mut Criterion) {

    c.bench_function("insert_and_calculate_rand_within_bound", |b| {
        let mut rng = Xoroshiro128::from_seed(&[1, 71, 1223]);

        let mut median_tracker = StreamingMedian::new(5);

        rng.gen_range(1, 10);
        b.iter(|| {
            median_tracker.insert_and_calculate(rng.gen());
        });
    });
}

criterion_group!(
    benches,
    bench_insert_and_calculate,
    bench_insert_and_calculate_rand,
    bench_insert_and_calculate_rand_within_bound
);

criterion_main!(benches);