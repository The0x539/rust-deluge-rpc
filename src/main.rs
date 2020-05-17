mod encoding;
mod rpc;
#[macro_use] mod session;
mod error;
mod receiver;
mod wtf;
use session::*;
use deluge_macro::*;

use serde::Deserialize;

fn read_file(path: &'static str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[derive(Deserialize, Debug, Query)]
struct Foo { name: String, num_files: u32 }

#[tokio::main()]
async fn main() {
    let mut session = Session::new(read_file("./experiment/endpoint")).await.unwrap();

    let daemon_version = session.daemon_info().await.unwrap();
    println!("Daemon version: {}", daemon_version);

    let user = read_file("./experiment/username");
    let pass = read_file("./experiment/password");
    let auth_level = session.login(&user, &pass).await.unwrap();
    println!("Auth level: {}", auth_level as u8);

    /*
    let filedump = std::fs::read("experiment/test.torrent").unwrap();
    let filedump2 = std::fs::read("experiment/shower.torrent").unwrap();
    let foo = session::TorrentOptions::default();
    let res = session.add_torrent_files(&[
        ("test.torrent", &base64::encode(filedump), &foo),
        ("shower.torrent", &base64::encode(filedump2), &foo),
    ]).await;
    match res {
        Ok(r) => println!("{:?}", r),
        Err(error::Error::Rpc(e)) => println!("{}", e),
        x => println!("{:?}", x),
    };
    */

    /*
    for id in session.get_session_state::<Vec<InfoHash>>().await.unwrap() {
        println!("{}", id);
        let q = session.get_torrent_status::<Foo>(&id).await.unwrap();
        println!("{:?}", q);
    }
    */

    println!("{}", serde_yaml::to_string(&session.get_filter_tree::<HashMap<_, _>>(true, &[]).await.unwrap()).unwrap());

    /*
    for method in session.get_method_list::<Vec<String>>().await.unwrap() {
        println!("{}", method);
    }
    */

    session.close().await.unwrap();
}
