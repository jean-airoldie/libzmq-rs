use super::*;

use criterion::{black_box, Benchmark, Criterion, Throughput};
use lazy_static::lazy_static;
use libzmq::{auth::*, prelude::*, *};
use rand::distributions::{Distribution, Standard};
use rand_core::SeedableRng;
use rand_isaac::Isaac64Rng;

lazy_static! {
    static ref ADDR: TcpAddr = "127.0.0.1:*".try_into().unwrap();
}

fn gen_dataset(dataset_size: usize, msg_size: usize) -> Vec<Vec<u8>> {
    let mut rng: Isaac64Rng = SeedableRng::seed_from_u64(123_490_814_327);
    (0..dataset_size)
        .map(|_| Standard.sample_iter(&mut rng).take(msg_size).collect())
        .collect()
}

pub(crate) fn bench(c: &mut Criterion) {
    c.bench(
        &"client-server 50u8 msg on TCP".to_owned(),
        Benchmark::new("dataset alloc (control)", move |b| {
            b.iter(|| {
                black_box(gen_dataset(MSG_AMOUNT, MSG_SIZE));
            });
        })
        .with_function("without CURVE encryption", move |b| {
            let producer = ServerBuilder::new()
                .bind(&*ADDR)
                .send_hwm(HWM)
                .build()
                .unwrap();

            let bound = producer.last_endpoint().unwrap();
            let consumer = ClientBuilder::new()
                .connect(bound)
                .recv_hwm(HWM)
                .build()
                .unwrap();

            consumer.send("").unwrap();
            let mut msg = producer.recv_msg().unwrap();
            let id = msg.routing_id().unwrap();

            b.iter(|| {
                let dataset = gen_dataset(MSG_AMOUNT, MSG_SIZE);
                for data in dataset {
                    let data: Msg = data.into();

                    producer.route(data, id).unwrap();
                    let _ = consumer.try_recv(&mut msg);
                }
            });
        })
        .with_function("with CURVE encryption", move |b| {
            let _ = AuthBuilder::new().no_curve_auth().build().unwrap();

            let server_cert = CurveCert::new_unique();

            let creds = CurveServerCreds::new(server_cert.secret());

            let producer = ServerBuilder::new()
                .bind(&*ADDR)
                .send_hwm(HWM)
                .mechanism(creds)
                .build()
                .unwrap();

            let bound = producer.last_endpoint().unwrap();

            let creds = CurveClientCreds::new(server_cert.public());

            let consumer = ClientBuilder::new()
                .connect(bound)
                .recv_hwm(HWM)
                .mechanism(creds)
                .build()
                .unwrap();

            consumer.send("").unwrap();
            let mut msg = producer.recv_msg().unwrap();
            let id = msg.routing_id().unwrap();

            b.iter(|| {
                let dataset = gen_dataset(MSG_AMOUNT, MSG_SIZE);
                for data in dataset {
                    let data: Msg = data.into();

                    producer.route(data, id).unwrap();
                    let _ = consumer.try_recv(&mut msg);
                }
            });
        })
        .throughput(Throughput::Bytes((MSG_AMOUNT * MSG_SIZE) as u64))
        .sample_size(SAMPLE_SIZE)
        .measurement_time(MEASUREMENT_TIME),
    );
}
