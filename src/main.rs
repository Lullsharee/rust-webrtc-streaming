extern crate ws;
extern crate rustc_serialize;
extern crate regex;
extern crate chan;
extern crate url;

use ws::{listen, Handler, Sender, Message, Handshake, CloseCode, Request, Response};
use std::rc::Rc;
use std::cell::RefCell;
use rustc_serialize::json::{self, ToJson, Json};
use regex::Regex;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;

struct OutSource {
    id: u32,
    data: String,
}

impl ToJson for OutSource {
    fn to_json(&self) -> Json {
        let mut bm = std::collections::BTreeMap::new();
        bm.insert("id".to_string(), self.id.to_json());
        bm.insert("data".to_string(), self.data.to_json());
        Json::Object(bm)
    }
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
struct Source {
    name: String,
    data: String,
}

#[derive(Clone)]
struct Stream {
    id: u32,
    name: String,
    ready: bool,
    viewers: Arc<Mutex<Vec<Sender>>>,
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
                    Ok(mut viewers) => (*viewers).push(client),
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
        let re = Regex::new(r"/view\?id=(?P<id>[0-9]+)").unwrap();
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
        let viewers = Arc::new(Mutex::new(Vec::new()));
        streamlist.streams.insert(
            self.id,
            Stream {
                id: self.id,
                name: "".to_string(),
                ready: false,
                viewers: viewers.clone(),
            },
        );
        let viewers = viewers.clone();
        let rx = self.rx.clone();
        let id = self.id;
        thread::spawn(move || {
            for frame in rx {
                let v = viewers.lock();
                match v {
                    Ok(mut viewers) => {
                        let mut dead = None;
                        for (pos, v) in (&*viewers).iter().enumerate() {
                            let out = OutSource {
                                id: id,
                                data: frame.clone(),
                            };
                            if !v.send(out.to_json().to_string()).is_ok() {
                                dead = Some(pos);
                            }
                        }
                        if let Some(d) = dead {
                            viewers.swap_remove(d);
                        }
                    }
                    _ => {
                        return;
                    }
                }
            }
            let viewers = viewers.lock();
            match viewers {
                Ok(viewers) => {
                    for v in &*viewers {
                        let _ = v.close(CloseCode::Normal);
                    }
                }
                _ => {}
            }
        });
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        if let Ok(text) = msg.into_text() {
            let f: Result<Source, _> = json::decode(&text);
            if let Ok(frame) = f {
                let mut streamlist = self.streamlist.borrow_mut();
                let s = streamlist.streams.get_mut(&self.id);
                match s {
                    Some(stream) => {
                        stream.name = frame.name;
                        stream.ready = true;
                        self.tx.send(frame.data);
                        Ok(())
                    }
                    _ => {
                        println!("Couldn't find stream ID | ID:{}", self.id);
                        self.client.close(CloseCode::Protocol)
                    }
                }
            } else {
                println!("couldn't decode json frame {}", text);
                self.client.close(CloseCode::Protocol)
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
                self.route_to(
                    req,
                    Box::new(Viewer {
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
