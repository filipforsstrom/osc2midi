extern crate rosc;

use rosc::{OscPacket};
use std::env;
use std::io::{stdin, stdout, Write};
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;

use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};

use anyhow::{Error, Result};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let usage = format!("Usage {} IP:PORT", &args[0]);
    if args.len() < 2 {
        println!("{}", usage);
        ::std::process::exit(1)
    }
    let addr = match SocketAddrV4::from_str(&args[1]) {
        Ok(addr) => addr,
        Err(_) => panic!("{}", usage),
    };
    let sock = UdpSocket::bind(addr)?;
    println!("Listening to {}", addr);

    let mut buf = [0u8; rosc::decoder::MTU];

    let midi_out = MidiOutput::new("Bus 1")?;

    let out_ports = midi_out.ports();
    let out_port: &MidiOutputPort = match out_ports.len() {
        0 => return Err(Error::msg("no output port found")),
        1 => {
            println!(
                "Choosing the only available output port: {}",
                midi_out.port_name(&out_ports[0]).unwrap()
            );
            &out_ports[0]
        }
        _ => {
            println!("\nAvailable output ports:");
            for (i, p) in out_ports.iter().enumerate() {
                println!("{}: {}", i, midi_out.port_name(p).unwrap());
            }
            print!("Please select output port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            out_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid output port selected")
                .unwrap()
        }
    };

    println!("\nOpening connection");
    let mut connection_out = midi_out.connect(out_port, "midir-test").unwrap();
    println!("Connection open. Listen!");

    Ok(loop {
        match sock.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                // println!("Received packet with size {} from: {}", size, _addr);
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size])?;
                handle_packet(packet, &mut connection_out)
            }
            Err(e) => {
                println!("Error receiving from socket: {}", e);
                break;
            }
        }
    })
}

fn handle_packet(packet: OscPacket, midi: &mut MidiOutputConnection) {
    let mut play_note = |note: u8, velocity: u8| {
        const NOTE_ON_MSG: u8 = 0x90;
        const NOTE_OFF_MSG: u8 = 0x80;
        const VELOCITY: u8 = 0x64;
        let _ = midi.send(&[NOTE_ON_MSG, note, velocity]);
    };

    match packet {
        OscPacket::Message(msg) => {
            // println!("OSC address: {}", msg.addr);
            // println!("OSC arguments: {:?}", msg.args);
            match msg.addr.as_str() {
                "/note" => {
                    let note = msg.args[0].clone().float().unwrap() as u8;
                    let velocity = msg.args[1].clone().float().unwrap() as u8;
                    play_note(note, velocity);
                }
                _ => {
                    println!("Unknown message received");
                }
            }
        }
        OscPacket::Bundle(_bundle) => {
            // println!("OSC Bundle: {:?}", _bundle);
        }
    }
}
