mod startup_args;
mod packet;
use packet::Packet;
mod error;

use dashmap::DashMap;
use std::{collections::HashMap, sync::{Arc}, time::Duration};

use clap::Parser;
use log::{debug, error, info, trace};
use tokio::{net::UdpSocket, sync::{RwLock, mpsc::{Receiver, Sender}}};
use rosc::{OscMessage, OscPacket};
use lazy_static::lazy_static;
lazy_static!{
    pub static ref TERMINATE: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
}

#[tokio::main]
async fn main() {
    eprintln!("Started main");
    //Set up logger
    simplelog::CombinedLogger::init(
        vec![
            simplelog::TermLogger::new(log::LevelFilter::Trace, simplelog::Config::default(), simplelog::TerminalMode::Mixed, simplelog::ColorChoice::Auto),
        ]
    ).unwrap();

    //Set up subscribers
    let subscribers: Arc<DashMap<String, DashMap<String, Sender<OscMessage>>>> = Arc::new(DashMap::new());

    //Parse CMD args
    let args = startup_args::StartupArgs::parse();

    const CHANNEL_SIZE: usize = 1024; //How many messages can be buffered for each subscriber while it is busy sending them out (otherwise receiver will stall)

    //Set up udp receiving
    //Parameters
    const UDP_RECEIVE_BUFFER_SIZE: usize = 1024 * 16; //limit of 16k, can be much much larger, but I think this suffices for the moment. Extra bytes get discarded
    //Create sockets
    let recv_addr = format!("{}:{}", args.bind_address, args.udp_receive_port);
    let recv_socket = UdpSocket::bind(recv_addr.clone()).await;
    let send_addr = format!("{}:{}", args.bind_address, args.udp_send_port);
    let send_socket = UdpSocket::bind(send_addr.clone()).await;
    
    let udp_subscribers = subscribers.clone();
    //Start new tokio task for receiving
    tokio::spawn(async move {
        //Check that sockets are set up correctly
        if send_socket.is_err(){
            error!("[UDP] Error starting sender on address {}: {:?}", send_addr, send_socket.unwrap_err());
            return;
        }
        let send_socket = send_socket.unwrap();

        if recv_socket.is_err(){
            error!("[UDP] Error starting listener on address {}: {:?}", recv_addr, recv_socket.unwrap_err());
            return;
        }
        let recv_socket = recv_socket.unwrap();

        //Create sender channel
        let (send_ch_sender, mut send_ch_receiver) = tokio::sync::mpsc::channel::<(std::net::SocketAddr, Vec<u8>)>(CHANNEL_SIZE);
        tokio::spawn(async move {
            loop{
                if let Some((target, data))= send_ch_receiver.recv().await{
                    if let Err(e) = send_socket.send_to(&data[..], target).await{
                        error!("[UDP] Error sending data to {:?}: {:?}", target,e);
                    }
                }else{
                    debug!("[UDP] Global sender thread for {:?} stopped", send_addr);
                    break;
                }
            }
        });
        
        //Receive buffer
        let mut buf = vec![0u8; UDP_RECEIVE_BUFFER_SIZE];
        loop{
        match recv_socket.recv_from(&mut buf).await{
            Ok((num_received, remote_addr)) => {
                debug!("[UDP] Received {} bytes from {:?}", num_received, remote_addr);
                match Packet::try_from(&buf[0..num_received]){
                    Ok(p) => {
                        match p{
                            Packet::Subscription(topic) => {
                                //Create new subscription
                                let (sub_send, mut sub_recv): (Sender<OscMessage>, Receiver<OscMessage>) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
                                let subscription_sender = send_ch_sender.clone();
                                tokio::spawn(async move{
                                    loop{
                                        if let Some(r) = sub_recv.recv().await{
                                            let bytes = rosc::encoder::encode(&OscPacket::Message(r));
                                            match bytes{
                                                Ok(b) => {
                                                    subscription_sender.send((remote_addr, b)).await.unwrap();
                                                },
                                                Err(e) => {
                                                    debug!("[UDP] Error encoding OSC packet: {:?}", e);
                                                },
                                            }
                                        }else{
                                            debug!("[UDP] Sender thread for {:?} stopped", remote_addr);
                                            break;
                                        }
                                    }
                                });
                                let remote_addr_descriptor = remote_addr.to_string();
                                info!("Added subscription to {} for remote endpoint {}", topic, remote_addr_descriptor);
                                udp_subscribers.entry(topic).or_default().insert(remote_addr_descriptor, sub_send);
                            },
                            Packet::Unsubscription(topic) => {
                                let remote_addr_descriptor = remote_addr.to_string();
                                info!("Removed subscription to {} for remote endpoint {}", topic, remote_addr_descriptor);
                                udp_subscribers.entry(topic).or_default().remove(&remote_addr_descriptor);
                            },
                            Packet::Normal(packets) => {
                                for p in packets{
                                    if let Some(subscribers) = udp_subscribers.get(&p.addr){
                                        for subscriber in subscribers.iter(){
                                            subscriber.send(p.clone()).await.unwrap();
                                        }
                                    }
                                }
                            },
                        }
                    },
                    Err(e) => {
                        error!("[UDP] OSC Error: {:?}", e)
                    }
                }
            },
            Err(e) => {
                error!("[UDP] Error receiving UDP packet: {:?}", e);   
            }
        }

    }});

    if let Some(websocket_port ) = args.websocket_port{

    }

    loop{
        tokio::time::sleep(Duration::from_millis(100)).await;
        if *TERMINATE.read().await == true{
            break;
        }
    }
}

#[cfg(test)]
mod tests;
