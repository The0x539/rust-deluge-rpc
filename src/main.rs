mod rencode;
#[macro_use] mod rpc;
mod session;
use session::Session;

use std::collections::HashMap;

use serde::Deserialize;

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[derive(Deserialize, Debug)]
struct Row {
    name: String,
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

    let statuses: HashMap<String, Row> = session.get_torrents_status(None, &["name"]).await.unwrap();
    for status in statuses {
        println!("{:?}", status);
    }

    session.close().await.unwrap();
}
