extern crate ws;
extern crate rustc_serialize;
extern crate regex;
extern crate url;
extern crate rand;


use ws::{listen, Handler, Sender, Message, Handshake, CloseCode, Request, Response};
use std::rc::Rc;
use std::cell::RefCell;
use rustc_serialize::json::{self, ToJson, Json};
use regex::Regex;
use std::thread::spawn;
use std::sync::{Arc, Mutex};
use std::collections::{BTreeMap, HashMap};
use rand::Rng;

trait CreateMessage {
    fn create_message(&self) -> String;
}

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

impl CreateMessage for AnswerSDP {
    fn create_message(&self) -> String {
        self.to_json().to_string()
    }
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

impl CreateMessage for ICE_cadinate {
    fn create_message(&self) -> String {
        self.to_json().to_string()
    }
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

enum ResMessage {
    Offer(OfferSDP),
    OfferMsg(Offertext),
    Answer(AnswerSDP),
    AnswerMsg(Source),
    IceCandinate(ICE_cadinate),
    Nomessage { text: String },
}
trait ToMessage {
    fn to_message(&self) -> ResMessage;
}

impl ToMessage for String {
    fn to_message(&self) -> ResMessage {
        let msg = match &self {
            req if self.contains("offer") => {
                let text: Offertext = json::decode(req).unwrap();
                ResMessage::OfferMsg(text)
            }
            req if self.contains("answer") => {
                let text: AnswerSDP = json::decode(req).unwrap();
                ResMessage::Answer(text)
            }
            req if self.contains("candidate") => {
                let text: ICE_cadinate = json::decode(req).unwrap();
                ResMessage::IceCandinate(text)
            }
            _ => {
                let text: Source = json::decode(&self).unwrap();
                ResMessage::AnswerMsg(text)
            }
        };
        msg
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
    sl: Arc<Mutex<StreamList>>,
}

impl Viewer {
    fn view_id(&mut self, id: u32) -> ws::Result<()> {
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
struct Hoster {
    client: Sender,
    streamlist: Arc<Mutex<StreamList>>,
    id: u32,
}

impl Handler for Hoster {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
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

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
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
                ResMessage::AnswerMsg(Source { name }) => {
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
    fn on_error(&mut self, _: ws::Error) {
        let mut streamlist = self.streamlist.lock().unwrap();
        let _ = streamlist.streams.remove(&self.id);
    }
}


struct ConnectList {
    client: Sender,
    sl: Arc<Mutex<StreamList>>,
}
impl Handler for ConnectList {
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        self.client.send(
            self.sl.lock().unwrap().to_json().to_string(),
        )
    }

    fn on_message(&mut self, _: Message) -> ws::Result<()> {
        self.client.close(CloseCode::Normal)
    }
}

struct Router {
    client: Sender,
    route: Box<Handler>,
    streamlist: Arc<Mutex<StreamList>>,
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
                let id = self.streamlist.lock().unwrap().new_id();

                self.route_to(
                    req,
                    Box::new(Hoster {
                        client: client,
                        streamlist: streamlist,
                        id: id,
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
