// use std::collections::HashMap;

use futures_util::{future, pin_mut, StreamExt};
// use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use std::fs::File;
//use std::io::prelude::*;
use std::io::BufReader;
// use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize as De, Serialize as Ser};

use http::Request as http_request;

#[derive(Ser, De)]
struct Settings {
    user: String,
    pw: String,
}

#[derive(Ser, De)]
#[serde(tag = "type")]
enum UpdateNotification {
    #[serde(rename = "friend-location")]
    FriendLocation { content: String },
}

#[derive(Debug)]
enum InstancePrivateLevel {
    Public,
    FriendsPlus,
    Friends,
    Private,
}

#[derive(Debug)]
struct ParsedLocation {
    user_id: String,
    display_name: String,
    world_name: String,
    private_level: InstancePrivateLevel,
    location: String,
}

fn load_settings() -> Settings {
    // let file_name = "settings.json";
    // let mut setting_file = File::open(file_name).expect("cannot find settings.json");
    // let mut raw_json = String::new();
    // setting_file.read_to_string(&mut raw_json).expect("cannot read settings.json");
    // serde_json::from_str(&raw_json).expect("cannot parse settings.json")

    serde_json::from_reader(BufReader::new(
        File::open("settings.json").expect("cannot find settings.json"),
    ))
    .expect("cannot parse settings.json")
}

fn parse_notifications(json_str: String) -> anyhow::Result<()> {
    let received_json: UpdateNotification = serde_json::from_str(&json_str)?;
    match received_json {
        UpdateNotification::FriendLocation { content } => {
            let maybe_parsed = |root: serde_json::Value| -> Option<ParsedLocation> {
                let user_id= root.get("userId")?.as_str()?.to_string();
                let display_name = root.get("user")?.get("displayName")?.as_str()?.to_string();
                let world_name = root.get("world")?.get("name")?.as_str()?.to_string();
                let location = root.get("location")?.as_str()?.to_string();
                Some(ParsedLocation{
                    user_id,
                    display_name,
                    world_name,
                    private_level : InstancePrivateLevel::Private,
                    location,
                })
            }(serde_json::from_str(&content)?);

            if let Some(parsed) = maybe_parsed {
                println!("{:?}", parsed);
            }

            // let hostname: Option<&str> = root
            //     .get("data")
            //     .and_then(|value| value.get(0))
            //     .and_then(|value| value.get("hostname"))
            //     .and_then(|value| value.as_str());

            // hostname is Some(string_value) if .data[0].hostname is a string,
            // and None if it was not found
            // println!("hostname = {:?}", hostname); // = Some("a hostname")
        }
        _ => (),
    };
    Ok(())
}

#[tokio::main]
async fn main() {
    let auth = test_request().await.unwrap();
    let connect_addr = format!("wss://vrchat.com/?authToken={}", auth);
    let url = url::Url::parse(&connect_addr).unwrap();

    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    println!("connect_addr = {}", connect_addr);
    let req = http_request::builder()
        .uri(connect_addr)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:94.0) Gecko/20100101 Firefox/94.0",
        )
        .body(())
        .unwrap();

    // let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (ws_stream, _) = connect_async(req).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();

            // tokio::io::stdout().write_all(&data).await.unwrap();
            // print!("\n\n");  // 改行しないと見えん…

            if let Ok(received_str) = String::from_utf8(data.to_vec()) {
                parse_notifications(received_str); // ここでのエラーは気にしない
                ()
            }
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}

async fn test_request() -> Result<String, Box<dyn std::error::Error>> {
    // https://vrchat.com/api/1/auth/user?apiKey=JlE5Jldo5Jibnk5O5hTx6XVqsJu4WJ26
    let settings = load_settings();
    let login_api = "https://vrchat.com/api/1/auth/user?apiKey=JlE5Jldo5Jibnk5O5hTx6XVqsJu4WJ26";
    dbg!("login_api url = {}", login_api);

    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:94.0) Gecko/20100101 Firefox/94.0",
        )
        .build()?;

    let response = client
        .get(login_api)
        //.basic_auth(settings.user.clone(), Some(settings.pw.clone()))
        .basic_auth(&settings.user, Some(settings.pw.clone()))
        .send()
        .await?;

    // for (key, val) in response.headers() {
    //     println!("{} : {:?}", key, val);
    // }

    // println!("----------------");
    // println!("status: {}", response.status());
    // println!("----------------");

    let auth = response
        .cookies()
        .find(|c| c.name() == "auth")
        .expect("Failed to lookup auth cookie from response");
    let auth = auth.value();

    println!("Auth: {}", auth);

    // println!("body: {:?}", response.bytes().await?);
    // let resp = reqwest::get("https://httpbin.org/ip")
    //     .await?
    //     .json::<HashMap<String, String>>()
    //     .await?;
    // println!("{:#?}", resp);
    Ok(auth.to_string())
}
