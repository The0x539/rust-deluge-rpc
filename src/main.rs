mod encoding;
mod rpc;
mod session;
mod receiver;
mod wtf;
mod types;

use deluge_rpc_macro::*;

use types::*;
use session::Session;

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[derive(Debug, serde::Deserialize, Query)]
struct Foo {
    label: String,
}

#[tokio::main()]
async fn main() {
    let mut session = Session::new(read_file("./experiment/endpoint")).await.unwrap();

    let daemon_version = session.daemon_info().await.unwrap();
    println!("Daemon version: {}", daemon_version);

    let user = read_file("./experiment/username");
    let pass = read_file("./experiment/password");
    let auth_level = session.login(&user, &pass).await.unwrap();
    println!("Auth level: {}", auth_level as u8);

    let x = session.get_torrents_status::<Foo>(None).await.unwrap();
    for (y, z) in x {
        println!("{}: {:?}", y, z);
    }

    let labels = session.get_labels().await.unwrap();
    for label in labels {
        println!("{}", label);
    }

    println!("{:?}", session.get_auth_levels_mappings().await.unwrap());

    session.close().await.unwrap();
}
