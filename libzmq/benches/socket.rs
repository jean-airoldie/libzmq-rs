use criterion::{black_box, Benchmark, Criterion, Throughput};

use libzmq::{addr::Endpoint, prelude::*, *};

use lazy_static::lazy_static;
use rand::{distributions::Standard, Rng};
use rand_core::SeedableRng;
use rand_isaac::Isaac64Rng;

use std::convert::TryInto;

const MSG_AMOUNT: usize = 1_000;
const MSG_SIZE: usize = 50;

lazy_static! {
    static ref ADDR: TcpAddr = "127.0.0.1:*".try_into().unwrap();
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
    c.bench(
        &format!("50u8 msg on TCP"),
        Benchmark::new("dataset alloc (control)", move |b| {
            b.iter(|| {
                black_box(gen_dataset(MSG_AMOUNT, MSG_SIZE));
            });
        })
        .with_function("server-client", move |b| {
            let producer = Server::new().unwrap();
            producer.bind(&*ADDR).unwrap();

            let bound = producer.last_endpoint().unwrap().unwrap();
            let consumer = Client::new().unwrap();
            consumer.connect(bound).unwrap();

            consumer.send("").unwrap();
            let mut msg = producer.recv_msg().unwrap();
            let routing_id = msg.routing_id().unwrap();

            b.iter(|| {
                let dataset = gen_dataset(MSG_AMOUNT, MSG_SIZE);
                for data in dataset {
                    let mut data: Msg = data.into();
                    data.set_routing_id(routing_id);

                    producer.send(data).unwrap();
                    let _ = consumer.try_recv(&mut msg);
                }
            });
        })
        .with_function("radio", move |b| {
            let producer = Radio::new().unwrap();
            producer.bind(&*ADDR).unwrap();


            let bound = producer.last_endpoint().unwrap().unwrap();
            let consumer = Dish::new().unwrap();
            consumer.connect(bound).unwrap();

            let mut msg = Msg::new();

            b.iter(|| {
                let dataset = gen_dataset(MSG_AMOUNT, MSG_SIZE);
                for data in dataset {
                    let mut data: Msg = data.into();
                    data.set_group(*GROUP);

                    producer.send(data).unwrap();
                    let _ = consumer.try_recv(&mut msg);
                }
            });
        })
        .throughput(Throughput::Bytes((MSG_AMOUNT * MSG_SIZE) as u32))
        .sample_size(30),
    );
}
