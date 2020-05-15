mod encoding;
mod rpc;
#[macro_use] mod session;
mod error;
mod receiver;
mod wtf;
use session::Session;

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

    let filedump = std::fs::read("experiment/test.torrent").unwrap();
    let filedump2 = std::fs::read("experiment/shower.torrent").unwrap();
    let res = session.add_torrent_files(&[
        ("test.torrent", &base64::encode(filedump), None),
        ("shower.torrent", &base64::encode(filedump2), None),
    ]).await;
    match res {
        Ok(r) => println!("{:?}", r),
        Err(error::Error::Rpc(e)) => println!("{}", e),
        x => println!("{:?}", x),
    };

    session.close().await.unwrap();
}
