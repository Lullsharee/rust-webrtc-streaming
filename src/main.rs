extern crate ws;
extern crate rustc_serialize;
extern crate regex;
extern crate chan;
extern crate url;
extern crate rand;


use ws::{listen, Handler, Sender, Message, Handshake, CloseCode, Request, Response};
use std::rc::Rc;
use std::cell::RefCell;
use rustc_serialize::json::{self, ToJson, Json};
use regex::Regex;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::{BTreeMap, HashMap};
use rand::Rng;

#[derive(RustcDecodable, RustcEncodable)]
struct Offertext {
    typed: String,
    sdp: String,
    stream_id: u32,
}

#[derive(RustcDecodable, RustcEncodable)]
struct OfferSDP {
    typed: String,
    sdp: String,
    client_id: u32,
}

impl ToJson for OfferSDP {
    fn to_json(&self) -> Json {
        let mut bm = BTreeMap::new();
        bm.insert("type".to_string(), self.typed.to_json());
        bm.insert("sdp".to_string(), self.sdp.to_json());
        bm.insert("client_id".to_string(), self.client_id.to_json());
        Json::Object(bm)
    }
}
#[derive(RustcDecodable, RustcEncodable)]
struct AnswerSDP {
    typed: String,
    sdp: String,
    client_id: u32,
}

impl ToJson for AnswerSDP {
    fn to_json(&self) -> Json {
        let mut mb = BTreeMap::new();
        mb.insert("type".to_string(), self.typed.to_json());
        mb.insert("sdp".to_string(), self.sdp.to_json());
        Json::Object(mb)
    }
}
#[derive(Debug, RustcDecodable, RustcEncodable)]
struct ICE_cadinate {
    typed: String,
    ice: ICE,
    id: u32,
}


impl ToJson for ICE_cadinate {
    fn to_json(&self) -> Json {
        let mut mb = BTreeMap::new();
        mb.insert("type".to_string(), self.typed.to_json());
        mb.insert("ice".to_string(), self.ice.to_json());
        mb.insert("stream_id".to_string(), self.id.to_json());
        Json::Object(mb)
    }
}
#[derive(Debug, RustcDecodable, RustcEncodable)]
struct ICE {
    candidate: String,
    sdpMid: String,
    sdpMLineIndex: u32,
}
impl ToJson for ICE {
    fn to_json(&self) -> Json {
        let mut mb = BTreeMap::new();
        mb.insert("candidate".to_string(), self.candidate.to_json());
        mb.insert("sdpMid".to_string(), self.sdpMid.to_json());
        mb.insert("sdpMLIneindex".to_string(), self.sdpMLineIndex.to_json());
        Json::Object(mb)
    }
}
#[derive(Debug, RustcDecodable, RustcEncodable)]
struct Source {
    name: String,
}

#[derive(Clone)]
struct Stream {
    id: u32,
    name: String,
    ready: bool,
    viewers: Arc<Mutex<BTreeMap<u32, Sender>>>,
    hoster: Sender,
}
impl ToJson for Stream {
    fn to_json(&self) -> Json {
        let mut bm = std::collections::BTreeMap::new();
        bm.insert("id".to_string(), self.id.to_json());
        bm.insert("name".to_string(), self.name.to_json());
        bm.insert("ready".to_string(), self.ready.to_json());
        Json::Object(bm)
    }
}

#[derive(Clone)]
struct StreamList {
    streams: BTreeMap<u32, Stream>,
    fresh_id: u32,
}

impl StreamList {
    fn new_id(&mut self) -> u32 {
        let id = self.fresh_id;
        self.fresh_id = self.fresh_id + 1;
        id
    }
}

impl ToJson for StreamList {
    fn to_json(&self) -> Json {
        let mut bm = std::collections::BTreeMap::new();
        let v: Vec<Stream> = self.streams.values().cloned().collect();
        bm.insert("streams".to_string(), v.to_json());
        Json::Object(bm)
    }
}

struct Viewer {
    id: u32,
    client: Sender,
    sl: Rc<RefCell<StreamList>>,
}

impl Viewer {
    fn view_id(&mut self, id: u32) -> ws::Result<()> {
        let mut sl = self.sl.borrow_mut();
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
    fn on_open(&mut self, hs: Handshake) -> ws::Result<()> {
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

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        if let Ok(text) = msg.into_text() {
            let f: Result<Offertext, _> = json::decode(&text);
            if let Ok(get_text) = f {
                let mut streamlist = self.sl.borrow_mut();
                let s_id = streamlist.streams.get_mut(&get_text.stream_id);
                match s_id {
                    Some(stream) => {
                        println!("success sdp");
                        let send_sdp = OfferSDP {
                            typed: get_text.typed,
                            sdp: get_text.sdp,
                            client_id: self.id,
                        };
                        stream.hoster.send(send_sdp.to_json().to_string());
                    }
                    _ => println!("not exist this steram id {}", &get_text.stream_id),
                }
            } else {
                let f: Result<ICE_cadinate, _> = json::decode(&text);
                if let Ok(get_text) = f {
                    let mut streamlist = self.sl.borrow_mut();
                    let s_id = streamlist.streams.get_mut(&get_text.id);
                    match s_id {
                        Some(stream) => {
                            println!("success cadinate");
                            let ice = ICE {
                                candidate: get_text.ice.candidate,
                                sdpMid: get_text.ice.sdpMid,
                                sdpMLineIndex: get_text.ice.sdpMLineIndex,
                            };
                            let ice_cadinate = ICE_cadinate {
                                typed: get_text.typed,
                                ice: ice,
                                id: get_text.id,
                            };
                            stream.hoster.send(ice_cadinate.to_json().to_string());
                        }
                        _ => println!("not exist this steram id {}", &get_text.id),
                    }
                } else {
                    println!("cant perse {}", &text);
                }
            }
        }
        Ok(())
    }
}
struct Hoster {
    client: Sender,
    streamlist: Rc<RefCell<StreamList>>,
    id: u32,
    tx: chan::Sender<String>,
    rx: chan::Receiver<String>,
}

impl Handler for Hoster {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        let mut streamlist = self.streamlist.borrow_mut();
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

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        if let Ok(text) = msg.into_text() {
            let f: Result<AnswerSDP, _> = json::decode(&text);
            if let Ok(frame) = f {
                let mut streamlist = self.streamlist.borrow_mut();
                let s = streamlist.streams.get_mut(&self.id);
                match s {
                    Some(stream) => {
                        let v = stream.viewers.lock();
                        match v {
                            Ok(mut viewers) => {
                                let viewer = viewers.get_mut(&frame.client_id);
                                match viewer {
                                    Some(out) => {
                                        out.send(frame.to_json().to_string());
                                    }

                                    _ => println!("not exist this client id {}", &frame.client_id),
                                }
                            }
                            _ => println!("Error: poisoned on Client iD{}", frame.client_id),
                        }
                        Ok(())
                    }
                    _ => {
                        println!("Couldn't find stream ID | ID:{}", self.id);
                        self.client.close(CloseCode::Protocol)
                    }
                }
            } else {
                let f: Result<Source, _> = json::decode(&text);
                if let Ok(frame) = f {
                    let mut streamlist = self.streamlist.borrow_mut();
                    let s = streamlist.streams.get_mut(&self.id);
                    match s {
                        Some(stream) => {
                            stream.name = frame.name;
                            stream.ready = true;
                            Ok(())
                        }
                        _ => {
                            println!("/host: couldn't find stream ID {}", self.id);
                            self.client.close(CloseCode::Protocol)
                        }
                    }
                } else {
                    let f: Result<ICE_cadinate, _> = json::decode(&text);
                    if let Ok(get_text) = f {
                        let mut streamlist = self.streamlist.borrow_mut();
                        let s_id = streamlist.streams.get_mut(&self.id);
                        match s_id {
                            Some(stream) => {
                                println!("success cadinate");
                                let v = stream.viewers.lock();
                                match v {
                                    Ok(mut viewers) => {
                                        let viewer = viewers.get_mut(&get_text.id);
                                        match viewer {
                                            Some(out) => {
                                                let ice = ICE {
                                                    candidate: get_text.ice.candidate,
                                                    sdpMid: get_text.ice.sdpMid,
                                                    sdpMLineIndex: get_text.ice.sdpMLineIndex,
                                                };

                                                let ice_cadinate = ICE_cadinate {
                                                    typed: get_text.typed,
                                                    ice: ice,
                                                    id: get_text.id,
                                                };
                                                out.send(ice_cadinate.to_json().to_string());
                                            }
                                            _ => {
                                                println!(
                                                    "not exist this client id {}",
                                                    &get_text.id
                                                )
                                            }
                                        }
                                        Ok(())
                                    }
                                    _ => {
                                        println!("Error: poisoned on Client iD{}", get_text.id);
                                        self.client.close(CloseCode::Protocol)
                                    }
                                }
                            }

                            _ => {
                                println!("not exist this steram id {}", &get_text.id);
                                self.client.close(CloseCode::Protocol)
                            }
                        }
                    } else {
                        println!("cant perse {}", &text);
                        self.client.close(CloseCode::Protocol)
                    }
                }
            }
        } else {
            println!("Couldn't get message");
            self.client.close(CloseCode::Protocol)
        }
    }
    fn on_close(&mut self, _: CloseCode, _: &str) {
        let mut streamlist = self.streamlist.borrow_mut();
        let _ = streamlist.streams.remove(&self.id);
    }
    fn on_error(&mut self, _: ws::Error) {
        let mut streamlist = self.streamlist.borrow_mut();
        let _ = streamlist.streams.remove(&self.id);
    }
}


struct ConnectList {
    client: Sender,
    sl: Rc<RefCell<StreamList>>,
}
impl Handler for ConnectList {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        self.client.send(self.sl.borrow().to_json().to_string())
    }

    fn on_message(&mut self, _: Message) -> ws::Result<()> {
        self.client.close(CloseCode::Normal)
    }
}

struct Router {
    client: Sender,
    route: Box<Handler>,
    streamlist: Rc<RefCell<StreamList>>,
}

impl Router {
    fn nothing_route(&mut self, req: &Request) -> ws::Result<Response> {
        let mut res = Response::from_request(req).unwrap();
        res.set_status(404);
        res.set_reason("404 Error Not found this route".to_string());
        Ok(res)
    }
    fn route_to(&mut self, req: &Request, r: Box<Handler>) -> ws::Result<Response> {
        self.route = r;
        Response::from_request(req)
    }
}
impl Handler for Router {
    fn on_request(&mut self, req: &Request) -> ws::Result<Response> {
        let (path_uri, path_value) = req.resource().split_at(5);
        let client = self.client.clone();
        let streamlist = self.streamlist.clone();
        let default = "".to_string();

        match path_uri {
            "/list" => {
                self.route_to(
                    req,
                    Box::new(ConnectList {
                        client: client,
                        sl: streamlist,
                    }),
                )
            }
            "/host" => {
                let (tx, rx) = chan::async();
                let id = self.streamlist.borrow_mut().new_id();

                self.route_to(
                    req,
                    Box::new(Hoster {
                        client: client,
                        streamlist: streamlist,
                        id: id,
                        tx: tx,
                        rx: rx,
                    }),
                )
            }
            "/view" => {
                let id = rand::thread_rng().gen_range(0, 1000);
                self.route_to(
                    req,
                    Box::new(Viewer {
                        id: id,
                        client: client,
                        sl: streamlist,
                    }),
                )
            }
            _ => self.nothing_route(req),
        }
    }
    fn on_open(&mut self, hs: Handshake) -> ws::Result<()> {
        self.route.on_open(hs)
    }

    fn on_shutdown(&mut self) {
        self.route.on_shutdown();
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        self.route.on_message(msg)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        self.route.on_close(code, reason);
    }
    fn on_error(&mut self, err: ws::Error) {
        self.route.on_error(err);
    }
}


fn main() {
    let sl = Rc::new(RefCell::new(StreamList {
        streams: BTreeMap::new(),
        fresh_id: 0,
    }));
    listen("127.0.0.1:3333", |client| {
        let cout = client.clone();
        Router {
            client: client,
            streamlist: sl.clone(),
            route: Box::new(move |_| cout.close(CloseCode::Error)),
        }
    }).unwrap();
}
