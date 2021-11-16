use rosc::{OscBundle, OscMessage, OscPacket};
use smallvec::SmallVec;

use crate::error::BoxError;

const SUBSCRIBE_ADDR: &str = "/subscribe";
const UNSUBSCRIBE_ADDR: &str = "/unsubscribe";

#[derive(Debug, Clone)]
pub enum Packet{
    Normal(SmallVec<[OscMessage; 1]>),
    Subscription(String),
    Unsubscription(String)
}

/*
As written right now, subscriptions and unsubscriptions cannot be bundled.
*/

impl TryFrom<&[u8]> for Packet{
    type Error = BoxError;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match rosc::decoder::decode(value)?{
            OscPacket::Message(m) => {
                if m.addr == SUBSCRIBE_ADDR {
                    if let Some(rosc::OscType::String(topic)) = m.args.get(0){
                        Ok(Packet::Subscription(topic.clone()))                        
                    }else{
                        Err(format!("Malformed subscription message: {:?}", m).into())
                    }
                } else if m.addr == UNSUBSCRIBE_ADDR {
                    if let Some(rosc::OscType::String(topic)) = m.args.get(0){
                        Ok(Packet::Unsubscription(topic.clone()))                        
                    }else{
                        Err(format!("Malformed unsubscription message: {:?}", m).into())
                    }
                } else {
                    Ok(Packet::Normal([m].into()))
                }
            },
            OscPacket::Bundle(b) => {
                let mut messages = SmallVec::new();
                fn process_bundle(bundle: OscBundle, messages: &mut SmallVec<[OscMessage; 1]>) {
                    for packet in bundle.content{
                        match packet{
                            OscPacket::Message(m) => messages.push(m),
                            OscPacket::Bundle(b) => process_bundle(b, messages),
                        }
                    }
                }
                process_bundle(b, &mut messages);
                Ok(Packet::Normal(messages))
            }
        }
    }
}