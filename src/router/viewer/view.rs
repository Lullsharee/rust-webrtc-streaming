use types::*;
use ws::{Handler, Sender, Message, Handshake, CloseCode, Result};
use rustc_serialize::json::ToJson;
use regex::Regex;
use std::sync::{Arc, Mutex};

pub struct Viewer {
    pub id: u32,
    pub client: Sender,
    pub sl: Arc<Mutex<StreamList>>,
}

impl Viewer {
    fn view_id(&mut self, id: u32) -> Result<()> {
        let mut sl = self.sl.lock().unwrap();
        let s_id = sl.streams.get_mut(&id);
        match s_id {
            Some(stream) => {
                let client = self.client.clone();
                let v = stream.viewers.lock();
                match v {

                    Ok(mut viewers) => {
                        (*viewers).insert(self.id, client);
                    }
                    _ => println!("Error: poisoned on Stream iD{}", id),
                };
                Ok(())
            }
            _ => {
                println!("Not exist this stream id {}", id);
                self.client.close(CloseCode::Protocol)
            }
        }
    }
}

impl Handler for Viewer {
    fn on_open(&mut self, hs: Handshake) -> Result<()> {
        let re = Regex::new(r"/view\?id=(?P<id>[0_9]+)").unwrap();
        let path = hs.request.resource();
        match re.captures(path) {
            Some(caps) => {
                match caps.get(1) {
                    Some(id_str) => {
                        match id_str.as_str().parse::<u32>() {
                            Ok(id) => self.view_id(id),
                            Err(_) => self.client.close(CloseCode::Policy),
                        }
                    }
                    None => self.client.close(CloseCode::Policy),
                }
            }
            None => self.client.close(CloseCode::Policy),
        }
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        if let Ok(text) = msg.into_text() {
            let req = text.to_message();
            match req {
                ResMessage::OfferMsg(Offertext {
                                         typed,
                                         sdp,
                                         stream_id,
                                     }) => {
                    let mut streamlist = self.sl.lock().unwrap();
                    let s_id = streamlist.streams.get_mut(&stream_id);
                    match s_id {
                        Some(stream) => {
                            println!("success sdp");
                            let send_sdp = OfferSDP {
                                typed: typed,
                                sdp: sdp,
                                client_id: self.id,
                            };
                            stream.hoster.send(send_sdp.to_json().to_string());
                        }
                        _ => println!("not exist this steram id {}", &stream_id),
                    }
                }
                ResMessage::IceCandinate(ICE_cadinate { typed, ice, id }) => {
                    let mut streamlist = self.sl.lock().unwrap();
                    let s_id = streamlist.streams.get_mut(&id);
                    match s_id {
                        Some(stream) => {
                            println!("success cadinate");
                            let res = ICE_cadinate {
                                typed: typed,
                                ice: ice,
                                id: id,
                            };
                            stream.hoster.send(res.create_message());
                        }
                        _ => println!("not exist this steram id {}", &id),
                    }
                }
                _ => println!("don't exist this message patter{}", &text),
            }
        } else {
            println!("Couldn't get message");
            self.client.close(CloseCode::Protocol);
        }
        Ok(())
    }
}
