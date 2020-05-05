mod rencode;
mod rpc;
mod session;

use session::Session;

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[tokio::main()]
async fn main() {
    let mut session = Session::new(read_file("./experiment/endpoint")).await.unwrap();

    // TODO: move auth into Session::new
    let user = read_file("./experiment/username");
    let pass = read_file("./experiment/password");
    let req = rpc_request!("daemon.login", [user, pass]);
    session.send(req).await.unwrap();

    let val = session.recv().await.unwrap();

    println!("{:?}", val);

    session.close().await.unwrap();
}
