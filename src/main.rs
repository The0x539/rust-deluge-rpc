mod rencode;
#[macro_use] mod rpc;
#[macro_use] mod session;
use session::Session;

use std::collections::HashMap;

use serde::Deserialize;

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

    #[derive(Deserialize, Debug)]
    struct Row { name: String }
    let filter = filter! { "name" => "archlinux-2020.05.01-x86_64.iso" };
    let statuses: HashMap<String, Row> = session.get_torrents_status(Some(filter), &["name"]).await.unwrap();
    for status in statuses {
        println!("{:?}", status);
    }

    session.close().await.unwrap();
}
