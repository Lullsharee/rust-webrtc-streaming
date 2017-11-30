use ws::Sender;
use rustc_serialize::json::{ToJson, Json};
use std::sync::{Arc, Mutex};
use std::collections::BTreeMap;


#[derive(Clone)]
pub struct Stream {
   pub id: u32,
   pub name: String,
   pub ready: bool,
   pub viewers: Arc<Mutex<BTreeMap<u32, Sender>>>,
   pub  hoster: Sender,
}
impl ToJson for Stream {
    fn to_json(&self) -> Json {
        let mut bm = BTreeMap::new();
        bm.insert("id".to_string(), self.id.to_json());
        bm.insert("name".to_string(), self.name.to_json());
        bm.insert("ready".to_string(), self.ready.to_json());
        Json::Object(bm)
    }
}

#[derive(Clone)]
pub struct StreamList {
   pub streams: BTreeMap<u32, Stream>,
   pub fresh_id: u32,
}

impl StreamList {
    pub fn new_id(&mut self) -> u32 {
        let id = self.fresh_id;
        self.fresh_id = self.fresh_id + 1;
        id
    }
}

impl ToJson for StreamList {
    fn to_json(&self) -> Json {
        let mut bm = BTreeMap::new();
        let v: Vec<Stream> = self.streams.values().cloned().collect();
        bm.insert("streams".to_string(), v.to_json());
        Json::Object(bm)
    }
}
