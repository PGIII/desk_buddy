use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    sync::mpsc::Sender,
};

use anyhow::anyhow;
use clap::Parser;
use db_link::commands::{Packet, PayloadBuf, MAX_PACKET_SIZE};

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
    let packets = [
        Packet::Echo(PayloadBuf::from_slice(b"bye").unwrap()).serialize(),
        Packet::GetParam(PayloadBuf::from_slice(b"VERSION").unwrap()).serialize(),
    ];
    for i in 0..2 {
        serial.write_all(&packets[i])?;
        let mut parser = db_link::parser::Parser::new();
        //read untill we get a packet or error
        loop {
            let mut read_buffer = [0u8; MAX_PACKET_SIZE];
            let bytes = serial.read(&mut read_buffer)?;
            if bytes > 0 {
                match parser.parse(&read_buffer) {
                    Ok(packet) => {
                        println!("Got: {packet:?}");
                        break;
                    }
                    Err(db_link::parser::Error::InvalidVersion) => {
                        return Err(anyhow!("Invalid protocol version"));
                    }
                    Err(_) => {
                        println!("{}", String::from_utf8_lossy(&read_buffer[..bytes]));
                    }
                }
            }
        }
    }
    Ok(())
}
