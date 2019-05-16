use criterion::{black_box, Benchmark, Criterion, Throughput};

use libzmq::{prelude::*, *};

use lazy_static::lazy_static;
use rand::{distributions::Standard, Rng};
use rand_core::SeedableRng;
use rand_isaac::Isaac64Rng;

use std::convert::TryInto;

const MSG_AMOUNT: usize = 100_000;
const MSG_SIZE_BYTES: [usize; 3] = [10, 50, 100];

lazy_static! {
    static ref INPROC: InprocAddr = "bench".try_into().unwrap();
    static ref GROUP: &'static Group = "group".try_into().unwrap();
}

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
            .with_function("server-client", move |b| {
                let producer = Server::new().unwrap();
                producer.bind(&*INPROC).unwrap();
                let consumer = Client::new().unwrap();
                consumer.connect(&*INPROC).unwrap();

                consumer.send("").unwrap();
                let mut msg = producer.recv_msg().unwrap();
                let routing_id = msg.routing_id().unwrap();

                b.iter(|| {
                    let dataset = gen_dataset(MSG_AMOUNT, *msg_size);
                    for data in dataset {
                        let mut data: Msg = data.into();
                        data.set_routing_id(routing_id);

                        producer.send(data).unwrap();
                        consumer.recv(&mut msg).unwrap();
                    }
                });

                producer.unbind(&*INPROC).unwrap();
                consumer.disconnect(&*INPROC).unwrap();
            })
            .with_function("radio", move |b| {
                let producer = Radio::new().unwrap();
                producer.bind(&*INPROC).unwrap();
                let consumer = Dish::new().unwrap();
                consumer.connect(&*INPROC).unwrap();

                let mut msg = Msg::new();

                b.iter(|| {
                    let dataset = gen_dataset(MSG_AMOUNT, *msg_size);
                    for data in dataset {
                        let mut data: Msg = data.into();
                        data.set_group(*GROUP);
                        producer.send(data).unwrap();
                        let _ = consumer.try_recv(&mut msg);
                    }
                });

                producer.unbind(&*INPROC).unwrap();
            })
            .with_function("mscp", move |b| {
                use std::sync::mpsc::channel;

                let (tx, rx) = channel();

                b.iter(|| {
                    let dataset = gen_dataset(MSG_AMOUNT, *msg_size);
                    for data in dataset {
                        tx.send(data).unwrap();
                        let data = rx.recv().unwrap();
                        black_box(data);
                    }
                });
            })
            .throughput(Throughput::Bytes((MSG_AMOUNT * msg_size) as u32))
            .sample_size(30),
        );
    }
}
