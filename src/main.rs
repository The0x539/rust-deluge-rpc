mod rencode;
#[macro_use] mod rpc;
mod session;
use session::Session;

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[tokio::main()]
async fn main() {
    let mut session = Session::new(read_file("./experiment/endpoint")).await.unwrap();

    let daemon_version = session.daemon_info().await.unwrap();
    println!("Daemon version: {}", daemon_version);

    let user = read_file("./experiment/username");
    let pass = read_file("./experiment/password");
    let auth_level = session.login(&user, &pass).await.unwrap();
    println!("Auth level: {}", auth_level);

    session.shutdown().await.unwrap();
}
