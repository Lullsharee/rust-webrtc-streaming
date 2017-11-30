use types::*;
use ws::{Result, Handshake, Handler, Sender, CloseCode, Message};
use std::sync::{Arc, Mutex};
use rustc_serialize::json::ToJson;
pub struct ConnectList {
    pub client: Sender,
    pub sl: Arc<Mutex<StreamList>>,
}
impl Handler for ConnectList {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        self.client.send(
            self.sl.lock().unwrap().to_json().to_string(),
        )
    }

    fn on_message(&mut self, _: Message) -> Result<()> {
        self.client.close(CloseCode::Normal)
    }
}
