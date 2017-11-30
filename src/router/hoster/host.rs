use types::*;

use ws::{Handler, Sender, Message, Handshake, CloseCode, Result, Error};
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;

pub struct Hoster {
    pub client: Sender,
    pub streamlist: Arc<Mutex<StreamList>>,
    pub id: u32,
}

impl Handler for Hoster {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        let mut streamlist = self.streamlist.lock().unwrap();
        let viewers = Arc::new(Mutex::new(BTreeMap::new()));
        let client = self.client.clone();
        streamlist.streams.insert(
            self.id,
            Stream {
                id: self.id,
                name: "".to_string(),
                ready: false,
                viewers: viewers.clone(),
                hoster: client,
            },
        );
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Ok(text) = msg.into_text() {
            let req = text.to_message();
            match req {
                ResMessage::Answer(AnswerSDP {
                                       typed,
                                       sdp,
                                       client_id,
                                   }) => {
                    let mut streamlist = self.streamlist.lock().unwrap();
                    let s = streamlist.streams.get_mut(&self.id);
                    match s {
                        Some(stream) => {
                            let v = stream.viewers.lock();
                            match v {
                                Ok(mut viewers) => {
                                    let viewer = viewers.get_mut(&client_id);
                                    match viewer {
                                        Some(out) => {
                                            println!("get answer");
                                            let res = AnswerSDP {
                                                typed: typed,
                                                sdp: sdp,
                                                client_id: client_id,
                                            };
                                            out.send(res.create_message());
                                        }
                                        _ => println!("not exist this client id {}", &client_id),
                                    }
                                }
                                _ => {
                                    println!("Error: poisoned on Client iD{}", client_id);
                                }
                            }
                            Ok(())
                        }
                        _ => {
                            println!("Couldn't find stream ID | ID:{}", self.id);
                            self.client.close(CloseCode::Protocol)
                        }
                    }
                }
                ResMessage::IceCandinate(ICE_cadinate { typed, ice, id }) => {
                    let mut streamlist = self.streamlist.lock().unwrap();
                    let s = streamlist.streams.get_mut(&self.id);
                    match s { 
                        Some(stream) => {
                            let v = stream.viewers.lock();
                            match v { 
                                Ok(mut viewers) => {
                                    let viewer = viewers.get_mut(&id);
                                    match viewer { 
                                        Some(out) => {
                                            println!("get hoster ice ");
                                            let res = ICE_cadinate {
                                                typed: typed,
                                                ice: ice,
                                                id: id,
                                            };
                                            out.send(res.create_message());
                                        }
                                        _ => println!("not exist this client id {}", id),
                                    }
                                    Ok(())
                                }
                                _ => {
                                    println!("Error: poisoned on Client iD{}", id);
                                    self.client.close(CloseCode::Protocol)
                                }
                            }
                        }
                        _ => {
                            println!("not exist this steram id {}", id);
                            self.client.close(CloseCode::Protocol)
                        }
                    }
                }             
                ResMessage::AnswerMsg(AnswerText { name }) => {
                    let mut streamlist = self.streamlist.lock().unwrap();
                    let s = streamlist.streams.get_mut(&self.id);
                    match s {
                        Some(stream) => {
                            stream.name = name;
                            stream.ready = true;
                            Ok(())
                        }
                        _ => {
                            println!("/host: couldn't find stream ID {}", self.id);
                            self.client.close(CloseCode::Protocol)
                        }
                    }
                }
                _ => self.client.close(CloseCode::Protocol),
            }
        } else {
            println!("Couldn't get message");
            self.client.close(CloseCode::Protocol)
        }
    }
    fn on_close(&mut self, _: CloseCode, _: &str) {
        let mut streamlist = self.streamlist.lock().unwrap();
        let _ = streamlist.streams.remove(&self.id);
    }
    fn on_error(&mut self, _: Error) {
        let mut streamlist = self.streamlist.lock().unwrap();
        let _ = streamlist.streams.remove(&self.id);
    }
}
