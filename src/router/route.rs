use types::*;
use router::lister::*;
use router::hoster::*;
use router::viewer::*;
use ws::{Handler, Sender, Message, Handshake, CloseCode, Request, Response, Result, Error};
use std::sync::{Arc, Mutex};
use rand::*;



pub struct Router {
    pub client: Sender,
    pub route: Box<Handler>,
    pub streamlist: Arc<Mutex<StreamList>>,
}
impl Router {
    fn nothing_route(&mut self, req: &Request) -> Result<Response> {
        let mut res = Response::from_request(req).unwrap();
        res.set_status(404);
        res.set_reason("404 Error Not found this route".to_string());
        Ok(res)
    }
    fn route_to(&mut self, req: &Request, r: Box<Handler>) -> Result<Response> {
        self.route = r;
        Response::from_request(req)
    }
}
impl Handler for Router {
    fn on_request(&mut self, req: &Request) -> Result<Response> {
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
                let id = thread_rng().gen_range(0, 1000);
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
    fn on_open(&mut self, hs: Handshake) -> Result<()> {
        self.route.on_open(hs)
    }

    fn on_shutdown(&mut self) {
        self.route.on_shutdown();
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        self.route.on_message(msg)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        self.route.on_close(code, reason);
    }
    fn on_error(&mut self, err: Error) {
        self.route.on_error(err);
    }
}
