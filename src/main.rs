mod encoding;
mod rpc;
#[macro_use] mod session;
mod error;
mod receiver;
mod wtf;
use session::Session;

use std::collections::HashMap;

use serde::Deserialize;

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[derive(Deserialize, Debug)]
struct Foo { name: String }
impl session::Query for Foo {
    fn keys() -> &'static [&'static str] { &["name"] }
}
#[derive(Deserialize, Debug)]
struct Bar { total_uploaded: u64 }
impl session::Query for Bar {
    fn keys() -> &'static [&'static str] { &["total_uploaded"] }
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

    let filter = filter! { "name" => "archlinux-2020.05.01-x86_64.iso" };
    let statuses: HashMap<String, Foo> = session.get_torrents_status(Some(filter)).await.unwrap();
    for (id, status) in statuses {
        println!("{:?}", status.name);
        let amount: Bar = session.get_torrent_status(&id).await.unwrap();
        println!("{}: {}", id, amount.total_uploaded);
    }

    session.close().await.unwrap();
}
