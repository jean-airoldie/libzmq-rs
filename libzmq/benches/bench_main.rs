use criterion::{criterion_group, criterion_main};

mod curve;
mod socket;
mod msg;

criterion_group!(benches, socket::bench, curve::bench, msg::bench);
criterion_main!(benches);
