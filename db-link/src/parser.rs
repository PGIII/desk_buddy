// originally addded this for memmem but not sure we need it, maybe its faster though
use crate::commands::{Command, Header, Packet, SYNC_BYTE, VERSION};
use fifo::Fifo;
use memchr::memchr;

#[derive(Debug)]
pub enum Error {
    InvalidVersion,
    NoSyncByte,
    InCompletePayload,
    InCompleteHeader,
}

const HEADER_SIZE: usize = core::mem::size_of::<Header>();

pub struct Parser {}
impl Parser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_buffer<'a>(&self, buffer: &'a [u8]) -> Result<Packet<'a>, Error> {
        //find sync byte
        for i in 0..buffer.len() {
            if buffer[i] == SYNC_BYTE {
                let packet_buf = &buffer[i..];
                return if packet_buf.len() >= HEADER_SIZE {
                    let header = buffer_to_header(packet_buf);
                    let payload = &packet_buf[HEADER_SIZE..];
                    if header.version == VERSION {
                        if payload.len() >= header.payload_length.into() {
                            Ok(parse_command(header.command, payload))
                        } else {
                            Err(Error::InCompletePayload)
                        }
                    } else {
                        Err(Error::InvalidVersion)
                    }
                } else {
                    Err(Error::InCompleteHeader)
                };
            }
        }

        Err(Error::NoSyncByte)
    }

    pub fn parse_fifo<'a>(
        &self,
        payload_buffer: &'a mut [u8; 255],
        fifo: &'a mut Fifo<u8>,
    ) -> Result<Packet<'a>, Error> {
        // advance to sync byte
        while let Some(b) = fifo.peek() {
            if b == SYNC_BYTE {
                if fifo.len() >= HEADER_SIZE {
                    let mut header_buf = [0u8; HEADER_SIZE];
                    fifo.read_to_buffer(&mut header_buf);
                    let header = buffer_to_header(&header_buf);
                    if header.version == VERSION {
                        if header.payload_length != 0 {
                            if fifo.len() >= header.payload_length as usize {
                                //this read should always succeed since we just checked size
                                fifo.read_to_buffer(payload_buffer);
                                return Ok(parse_command(header.command, payload_buffer));
                            } else {
                                return Err(Error::InCompletePayload);
                            }
                        }
                    } else {
                        return Err(Error::InvalidVersion);
                    }
                } else {
                    return Err(Error::InCompleteHeader);
                }
            } else {
                _ = fifo.read();
            }
        }
        Err(Error::NoSyncByte)
    }
}

fn parse_command<'a>(command: Command, payload: &'a [u8]) -> Packet<'a> {
    match command {
        Command::Echo => Packet::Echo(payload),
        Command::Version => Packet::Version,
        Command::Ping => Packet::Ping,
    }
}

fn buffer_to_header<'a>(buffer: &'a [u8]) -> &'a Header {
    let (head, body, _tail) = unsafe { buffer.align_to::<Header>() };
    assert!(head.is_empty(), "Error Casting buf to header");
    &body[0]
}

#[cfg(test)]
mod tests {
    use crate::commands::Command;
    use crate::commands::{SYNC_BYTE, VERSION};

    use super::*;
    use paste;

    #[test]
    pub fn test_parse_buffer() {
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
        let parser = Parser::new();
        let output = parser.parse_buffer(&buffer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }
}
