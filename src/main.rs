extern crate ws;
extern crate webrtc_live;

use ws::{listen, CloseCode};
use std::thread::spawn;
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;
use webrtc_live::router::route::Router;
use webrtc_live::types::StreamList;


fn main() {
    let sl = Arc::new(Mutex::new(StreamList {
        streams: BTreeMap::new(),
        fresh_id: 0,
    }));
    let server = spawn(move || {
        listen("127.0.0.1:3333", |client| {
            let cout = client.clone();
            Router {
                client: client,
                streamlist: sl.clone(),
                route: Box::new(move |_| cout.close(CloseCode::Error)),
            }
        }).unwrap()
    });
    let _ = server.join();
}
