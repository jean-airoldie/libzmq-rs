use criterion::{criterion_group, criterion_main};

mod socket;

criterion_group!(benches, socket::bench);
criterion_main!(benches);
