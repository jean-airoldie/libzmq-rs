use super::*;

use criterion::{black_box, Benchmark, Criterion, Throughput};
use lazy_static::lazy_static;
use libzmq::{prelude::*, *};
use rand::{distributions::Standard, Rng};
use rand_core::SeedableRng;
use rand_isaac::Isaac64Rng;

lazy_static! {
    static ref ADDR: TcpAddr = "127.0.0.1:*".try_into().unwrap();
    static ref GROUP: Group = "group".try_into().unwrap();
}

fn gen_dataset(dataset_size: usize, msg_size: usize) -> Vec<Vec<u8>> {
    let mut rng: Isaac64Rng = SeedableRng::seed_from_u64(123_490_814_327);
    let mut dataset = Vec::with_capacity(dataset_size);

    for _ in 0..dataset_size {
        let s: Vec<u8> = rng.sample_iter(&Standard).take(msg_size).collect();
        dataset.push(s);
    }

    dataset
}

pub(crate) fn bench(c: &mut Criterion) {
    c.bench(
        &"50u8 msg on TCP".to_owned(),
        Benchmark::new("dataset alloc (control)", move |b| {
            b.iter(|| {
                black_box(gen_dataset(MSG_AMOUNT, MSG_SIZE));
            });
        })
        .with_function("server-client", move |b| {
            let producer = ServerBuilder::new()
                .bind(&*ADDR)
                .send_hwm(HWM)
                .build()
                .unwrap();

            let bound = producer.last_endpoint().unwrap().unwrap();
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
        .with_function("radio-dish", move |b| {
            let producer = RadioBuilder::new()
                .bind(&*ADDR)
                .send_hwm(HWM)
                .build()
                .unwrap();

            let bound = producer.last_endpoint().unwrap().unwrap();
            let consumer = DishBuilder::new()
                .connect(bound)
                .recv_hwm(HWM)
                .build()
                .unwrap();

            let mut msg = Msg::new();

            b.iter(|| {
                let dataset = gen_dataset(MSG_AMOUNT, MSG_SIZE);
                for data in dataset {
                    let data: Msg = data.into();

                    producer.transmit(data, &*GROUP).unwrap();
                    let _ = consumer.try_recv(&mut msg);
                }
            });
        })
        .with_function("scatter-gather", move |b| {
            let producer = ScatterBuilder::new()
                .bind(&*ADDR)
                .send_hwm(HWM)
                .build()
                .unwrap();

            let bound = producer.last_endpoint().unwrap().unwrap();
            let consumer = GatherBuilder::new()
                .connect(bound)
                .recv_hwm(HWM)
                .build()
                .unwrap();

            let mut msg = Msg::new();

            b.iter(|| {
                let dataset = gen_dataset(MSG_AMOUNT, MSG_SIZE);
                for data in dataset {
                    let data: Msg = data.into();

                    producer.send(data).unwrap();
                    let _ = consumer.try_recv(&mut msg);
                }
            });
        })
        .throughput(Throughput::Bytes((MSG_AMOUNT * MSG_SIZE) as u32))
        .sample_size(SAMPLE_SIZE)
        .measurement_time(Duration::from_secs(30)),
    );
}
