use std::time::Duration;

use log::*;
use rosc::{OscMessage, OscPacket, OscType};
use tokio::net::UdpSocket;

use crate::main;
use crate::TERMINATE;

#[tokio::test]
async fn test_single_sender_single_receiver(){
    info!("[TEST] Starting main");
    std::thread::spawn(|| {
        main();
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    info!("[TEST] Setting up channel");
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<Result<String, String>>(1);

    info!("[TEST] Setting up receiver");
    let recv_addr = "127.0.0.1:12000";
    tokio::spawn(async move{
        let recv_socket = UdpSocket::bind(recv_addr).await.unwrap();
        //subscribe
        let subscription = rosc::encoder::encode(
            &OscPacket::Message(
                OscMessage{ 
                    addr: "/subscribe".to_string(), 
                    args: vec!["/a".into()] 
                }
            )
        ).unwrap();
        recv_socket.send_to(&subscription, "127.0.0.1:9000").await.unwrap();
        //receive
        debug!("[TEST:RECV] Waiting for buffer");
        let mut recv_buffer = vec![0u8; 16 * 1024];
        let num_received = recv_socket.recv(&mut recv_buffer).await.unwrap();
        debug!("[TEST:RECV] Received {} bytes", num_received);
        let packet = rosc::decoder::decode(&recv_buffer[0..num_received]).unwrap();
        if let OscPacket::Message(m) = packet{
            if let Some(OscType::String(s)) = m.args.get(0){
                sender.send(Ok(s.to_string())).await.unwrap();
            }else{
                sender.send(Err("Received the wrong packet".to_string())).await.unwrap();
            }
        }else{
            sender.send(Err("Received the wrong packet".to_string())).await.unwrap();
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    info!("[TEST] Setting up sender");
    let send_addr = "127.0.0.1:12001";
    tokio::spawn(async move{
        let send_socket  = UdpSocket::bind(send_addr).await.unwrap();
        //subscribe
        let message = rosc::encoder::encode(
            &OscPacket::Message(
                OscMessage{ 
                    addr: "/a".to_string(), 
                    args: vec!["b".into()] 
                }
            )
        ).unwrap();
        send_socket.send_to(&message, "127.0.0.1:9000").await.unwrap();
        debug!("[TEST:SEND] Sent message");
    }).await.unwrap();

    info!("[TEST] Waiting for value");
    let value = receiver.recv().await.unwrap().unwrap();
    debug!("[TEST] Received value {:?}", value);
    assert!(value == "b");

    info!("[TEST] Shutting down main loop");
    *TERMINATE.write().await = true;
}   