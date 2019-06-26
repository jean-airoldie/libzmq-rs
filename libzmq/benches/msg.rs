use super::*;

use criterion::{black_box, Benchmark, Criterion};
use libzmq::Msg;

pub(crate) fn bench(c: &mut Criterion) {
    c.bench(
        &"Msg::from".to_owned(),
        Benchmark::new("String", move |b| {
            b.iter(|| {
                let string = "some string".to_owned();
                let msg: Msg = string.into();
                black_box(msg);
            });
        })
        .with_function("&str", move |b| {
            b.iter(|| {
                let msg: Msg = "some str".into();
                black_box(msg);
            });
        })
        .with_function("Vec<u8>", move |b| {
            b.iter(|| {
                let bytes = b"some bytes".to_owned();
                let msg: Msg = bytes.into();
                black_box(msg);
            });
        })
        .with_function("&[u8; X]", move |b| {
            b.iter(|| {
                let msg: Msg = b"some bytes".into();
                black_box(msg);
            });
        })
        .with_function("&[u8]", move |b| {
            b.iter(|| {
                let bytes: &[u8] = b"some bytes";
                let msg: Msg = bytes.into();
                black_box(msg);
            });
        })
        .measurement_time(MEASUREMENT_TIME),
    );
}
