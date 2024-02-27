use ewebsock::WsMessage;

#[tokio::main]
async fn main() {
    let wakeup = move || {            println!(" loki");
    }; // wake up UI thread on new message
    match ewebsock::connect_with_wakeup("ws://127.0.0.1:3030/tsurust-ws/", wakeup )  {
        Ok((mut ws_sender, ws_receiver)) => {
            println!(" oki");
            ws_sender.send(WsMessage::Text("waa".to_string()));
            println!(" doki");

            while let Some(event) = ws_receiver.try_recv() {
                dbg!(" {}", event);
            }

            loop {

            }
        }
        Err(error) => {
            dbg!(" noki {}", error);
        }
    }


}
