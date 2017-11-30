use std::collections::BTreeMap;
use rustc_serialize::json::{self, ToJson, Json};


pub enum ResMessage {
    Offer(OfferSDP),
    OfferMsg(Offertext),
    Answer(AnswerSDP),
    AnswerMsg(AnswerText),
    IceCandinate(ICE_cadinate),
    Nomessage { text: String },
}
pub trait ToMessage {
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
                let text: AnswerText = json::decode(&self).unwrap();
                ResMessage::AnswerMsg(text)
            }
        };
        msg
    }
}

pub trait CreateMessage {
    fn create_message(&self) -> String;
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct Offertext {
   pub typed: String,
   pub sdp: String,
   pub stream_id: u32,
}


#[derive(RustcDecodable, RustcEncodable)]
pub struct OfferSDP {
   pub typed: String,
   pub sdp: String,
   pub client_id: u32,
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
pub struct AnswerSDP {
   pub typed: String,
   pub sdp: String,
   pub client_id: u32,
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
pub struct ICE_cadinate {
   pub typed: String,
   pub ice: ICE,
   pub id: u32,
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
pub struct ICE {
   pub candidate: String,
   pub sdpMid: String,
   pub sdpMLineIndex: u32,
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
pub struct AnswerText {
   pub name: String,
}
