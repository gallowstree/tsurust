use std::collections::HashMap;
use std::sync::Arc;
use futures::stream::SplitStream;
use futures::StreamExt;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::Sender;
use warp::Filter;
use warp::ws::{Message, WebSocket};
use tsurust_common::board::PlayerID;

type Clients = Arc<RwLock<HashMap<PlayerID, mpsc::UnboundedSender<Message>>>>;


#[tokio::main]
async fn main() {
    let clients = Clients::default();

    eprintln!("hapi");


    let ws_route = warp::path("tsurust-ws")
        .and(warp::ws())
        //.and(warp::any().map(move || clients.clone()))
        .map(move |ws: warp::ws::Ws| {
            ws.on_upgrade(|socket| client_connected(socket))
        });

    warp::serve(ws_route)
        .run(([127, 0, 0, 1], 3030)).await;
}

async fn client_connected(ws: WebSocket) {
    let (ws_out, mut ws_in) = ws.split(); //maybe spawn an outbound task?
    eprintln!("clioent connected");

    client_message_loop(ws_in).await;

    dbg!("player disconnected");
}

async fn client_message_loop(mut ws_in: SplitStream<WebSocket>) {
    while let Some(result) = ws_in.next().await { //and_then()?
        match result {
            Ok(msg) => dbg!(msg),
            Err(e) => {
                eprintln!("websocket error: {}", e);
                break;
            }
        };
    }
}

