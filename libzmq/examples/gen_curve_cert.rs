use libzmq::auth::CurveCert;

fn main() {
    let cert = CurveCert::new_unique();
    println!("{:#?}", cert);
}
