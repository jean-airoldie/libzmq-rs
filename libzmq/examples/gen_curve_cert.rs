use libzmq::auth::CurveCert;

// Used to generate `CURVE` certificates.
fn main() {
    let cert = CurveCert::new_unique();
    println!("public: \"{}\"", cert.public().as_str());
    println!("secret: \"{}\"", cert.secret().as_str());
}
