use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
};

use anyhow::anyhow;
use clap::Parser;
use db_link::commands::{Packet, MAX_PACKET_SIZE};

#[derive(Parser, Default)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    serial_port_path: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let mut serial = OpenOptions::new()
        .read(true)
        .write(true)
        .open(args.serial_port_path)?;
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    let packet = Packet::Echo(b"Hello There");
    serial.write_all(packet.serialize(&mut buffer))?;
    let mut parser = db_link::parser::Parser::new();
    //read untill we get a packet or error
    loop {
        let mut read_buffer = [0u8; MAX_PACKET_SIZE];
        serial.read(&mut read_buffer)?;
        match parser.parse(&read_buffer) {
            Ok(packet) => {
                println!("Got: {packet:?}");
                break;
            }
            Err(db_link::parser::Error::InvalidVersion) => {
                return Err(anyhow!("Invalid protocol version"));
            }
            Err(_) => {}
        }
    }
    Ok(())
}
