use libzmq::auth::CurveCert;

// Used to generate `CURVE` certificates.
fn main() {
    let cert = CurveCert::new_unique();
    println!("{:#?}", cert);
}
