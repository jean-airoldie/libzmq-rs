use criterion::{criterion_group, criterion_main};

mod curve;
mod msg;
mod socket;

criterion_group!(benches, socket::bench, curve::bench, msg::bench);
criterion_main!(benches);
