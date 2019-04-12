use criterion::{black_box, Benchmark, Criterion, Throughput};

use rand::{
    distributions::{Alphanumeric, Standard},
    Rng,
};
use rand_core::SeedableRng;
use rand_isaac::Isaac64Rng;

use libzmq::prelude::*;

use std::{thread, time::Duration};

const MSG_AMOUNT: usize = 1_000_000;
const INPROC_ADDR: &str = "inproc://bench";
const MSG_SIZE_BYTES: [usize; 3] = [10, 50, 100];

fn gen_dataset(dataset_size: usize, msg_size: usize) -> Vec<Vec<u8>> {
    let mut rng: Isaac64Rng = SeedableRng::seed_from_u64(123490814327);
    let mut dataset = Vec::with_capacity(dataset_size);

    for _ in 0..dataset_size {
        let s: Vec<u8> = rng.sample_iter(&Standard).take(msg_size).collect();
        dataset.push(s);
    }

    dataset
}

pub(crate) fn bench(c: &mut Criterion) {
    for msg_size in &MSG_SIZE_BYTES {
        c.bench(
            &format!("msg_size: {} bytes, transport: inproc", msg_size),
            Benchmark::new("control", move |b| {
                b.iter(|| {
                    black_box(gen_dataset(MSG_AMOUNT, *msg_size));
                });
            })
            .with_function("client-client", move |b| {
                let endpoint: Endpoint = INPROC_ADDR.parse().unwrap();

                let producer = Client::new().unwrap();
                producer.bind(&endpoint).unwrap();
                let consumer = Client::new().unwrap();
                consumer.connect(&endpoint).unwrap();

                let mut msg = Msg::new();

                b.iter(|| {
                    let dataset = gen_dataset(MSG_AMOUNT, *msg_size);
                    for data in dataset {
                        producer.send(data).unwrap();
                        consumer.recv(&mut msg).unwrap();
                    }
                });

                producer.unbind(&endpoint).unwrap();
                consumer.disconnect(&endpoint).unwrap();
            })
            .with_function("server-client", move |b| {
                let endpoint: Endpoint = INPROC_ADDR.parse().unwrap();

                let producer = Server::new().unwrap();
                producer.bind(&endpoint).unwrap();
                let consumer = Client::new().unwrap();
                consumer.connect(&endpoint).unwrap();

                consumer.send("").unwrap();
                let mut msg = producer.recv_msg().unwrap();
                let routing_id = msg.routing_id().unwrap();

                b.iter(|| {
                    let dataset = gen_dataset(MSG_AMOUNT, *msg_size);
                    for data in dataset {
                        let mut data: Msg = data.into();
                        data.set_routing_id(routing_id).unwrap();

                        producer.send(data).unwrap();
                        consumer.recv(&mut msg).unwrap();
                    }
                });

                producer.unbind(&endpoint).unwrap();
                consumer.disconnect(&endpoint).unwrap();
            })
            .with_function("radio", move |b| {
                let endpoint: Endpoint = INPROC_ADDR.parse().unwrap();

                let producer = Radio::new().unwrap();
                producer.bind(&endpoint).unwrap();

                b.iter(|| {
                    let dataset = gen_dataset(MSG_AMOUNT, *msg_size);
                    for data in dataset {
                        let mut data: Msg = data.into();
                        data.set_group("group").unwrap();
                    }
                });

                producer.unbind(&endpoint).unwrap();
            })
            .throughput(Throughput::Bytes((MSG_AMOUNT * msg_size) as u32))
            .sample_size(10),
        );
    }
}
