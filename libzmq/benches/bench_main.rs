use criterion::{criterion_group, criterion_main};

mod curve;
mod socket;

criterion_group!(benches, socket::bench, curve::bench);
criterion_main!(benches);
