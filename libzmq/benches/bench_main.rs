mod curve;
mod msg;
mod socket;

use criterion::{criterion_group, criterion_main};

use std::time::Duration;

const MSG_AMOUNT: usize = 100_000;
const MSG_SIZE: usize = 50;
const HWM: i32 = 1_000;
const SAMPLE_SIZE: usize = 10;
const MEASUREMENT_TIME: Duration = Duration::from_secs(30);

criterion_group!(benches, socket::bench, curve::bench, msg::bench);
criterion_main!(benches);
