// originally addded this for memmem but not sure we need it, maybe its faster though
use crate::commands::{Command, Header, Packet, SYNC_BYTE, VERSION};
use memchr::memchr;

#[derive(Debug)]
pub enum Error {
    InvalidVersion,
    NoSyncByte,
    InCompletePayload,
    InCompleteHeader,
}

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
                const HEADER_SIZE: usize = core::mem::size_of::<Header>();
                return if packet_buf.len() >= HEADER_SIZE {
                    let (head, body, _tail) = unsafe { packet_buf.align_to::<Header>() };
                    assert!(head.is_empty(), "Error Casting buf to header");
                    let header = &body[0];
                    let payload = &packet_buf[HEADER_SIZE..];
                    if header.version == VERSION {
                        if payload.len() >= header.payload_length.into() {
                            match header.command {
                                Command::Echo => Ok(Packet::Echo(payload)),
                                Command::Version => Ok(Packet::Version),
                                Command::Ping => Ok(Packet::Ping),
                            }
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
}

#[cfg(test)]
mod tests {
    use crate::commands::Command;
    use crate::commands::{SYNC_BYTE, VERSION};

    use super::*;
    use paste;

    #[test]
    pub fn test_parse_buffer() {
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, 0, b'h', b'i'];
        let parser = Parser::new();
        let output = parser.parse_buffer(&buffer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }
}
