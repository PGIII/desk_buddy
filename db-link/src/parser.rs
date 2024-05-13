use crate::commands::{Command, Header, Packet, HEADER_SIZE, MAX_PACKET_SIZE, SYNC_BYTE, VERSION};

#[cfg(feature = "std")]
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Error))]
pub enum Error {
    #[cfg_attr(feature = "std", error("invalid protocol version"))]
    InvalidVersion,
    #[cfg_attr(feature = "std", error("no sync byte found"))]
    NoSyncByte,
    #[cfg_attr(feature = "std", error("incomplete payload"))]
    InCompletePayload,
    #[cfg_attr(feature = "std", error("incomplete header"))]
    InCompleteHeader,
}

#[derive(Debug, Clone, Copy)]
pub enum Status {
    WaitingForSync,
    WaitingForHeader,
    WaitingForPayload(Command, usize), //command, and payload length
}

#[derive(Debug, Clone, Copy)]
pub struct Parser {
    status: Status,
    buffer: [u8; MAX_PACKET_SIZE],
    buffer_pos: usize,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            status: Status::WaitingForSync,
            buffer_pos: 0,
            buffer: [0u8; MAX_PACKET_SIZE],
        }
    }

    /// Stores buffer in internal buffer, returning a packet if found
    /// Note: Some potential Foot guns.
    ///     Due do this being optimized for embedded enviorments without an allocator (heap) the
    ///     returned packet may reference the internal buffer of the parser
    ///     as such packets should be used or the buffer they reference copied before pushing more
    ///     bytes to the parser
    pub fn parse(&mut self, buffer: &[u8]) -> Result<Packet, Error> {
        for b in buffer {
            //read a byte and see if we need to advance the state machine
            self.buffer[self.buffer_pos] = *b;
            self.buffer_pos += 1;
            match self.status {
                Status::WaitingForSync => {
                    if self.buffer[0] == SYNC_BYTE {
                        self.status = Status::WaitingForHeader;
                    } else {
                        self.buffer_pos = 0;
                    }
                }
                Status::WaitingForHeader => {
                    if self.buffer_pos == HEADER_SIZE {
                        let header = buffer_to_header(&self.buffer[..HEADER_SIZE]);
                        if header.version == VERSION {
                            self.status = Status::WaitingForPayload(
                                header.command,
                                header.payload_length as usize,
                            );
                        } else {
                            return Err(Error::InvalidVersion);
                        }
                    }
                }
                Status::WaitingForPayload(command, payload_len) => {
                    if self.buffer_pos == HEADER_SIZE + payload_len {
                        self.buffer_pos = 0;
                        self.status = Status::WaitingForSync;
                        return Ok(parse_command(
                            command,
                            &self.buffer[HEADER_SIZE..HEADER_SIZE + payload_len],
                        ));
                    }
                }
            }
        }

        match self.status {
            Status::WaitingForSync => Err(Error::NoSyncByte),
            Status::WaitingForHeader => Err(Error::InCompleteHeader),
            Status::WaitingForPayload(_, _) => Err(Error::InCompletePayload),
        }
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

    #[test]
    pub fn test_parse_buffer() {
        let buffer = [
            SYNC_BYTE,
            VERSION,
            Command::Echo as u8,
            5,
            b'h',
            b'e',
            b'l',
            b'l',
            b'o',
        ];
        let mut parser = Parser::new();
        let output = parser.parse(&buffer).unwrap();
        assert_eq!(output, Packet::Echo(b"hello"));
    }

    #[test]
    pub fn test_multi_parse() {
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
        let mut parser = Parser::new();
        let output = parser.parse(&buffer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
        let buffer2 = [SYNC_BYTE, VERSION, Command::Echo as u8, 3, b'b', b'y', b'e'];
        let output2 = parser.parse(&buffer2).unwrap();
        assert_eq!(output2, Packet::Echo(b"bye"));
    }

    #[test]
    pub fn test_unaligned_parse() {
        let buffer = [
            b'o',
            b'h',
            SYNC_BYTE,
            VERSION,
            Command::Echo as u8,
            5,
            b'h',
            b'e',
            b'l',
            b'l',
            b'o',
        ];
        let mut parser = Parser::new();
        let output = parser.parse(&buffer).unwrap();
        assert_eq!(output, Packet::Echo(b"hello"));
    }

    #[test]
    pub fn test_partial_header() {
        let buffer = [SYNC_BYTE, VERSION];
        let mut parser = Parser::new();
        if let Err(output) = parser.parse(&buffer) {
            assert_eq!(output, Error::InCompleteHeader);
        } else {
            assert!(false, "got ok but expected error")
        }
    }

    #[test]
    pub fn test_wrong_version() {
        let buffer = [SYNC_BYTE, VERSION + 1, Command::Version as u8, 0];
        let mut parser = Parser::new();
        if let Err(output) = parser.parse(&buffer) {
            assert_eq!(output, Error::InvalidVersion);
        } else {
            assert!(false, "got ok but expected error")
        }
    }

    #[test]
    pub fn test_no_sync() {
        let buffer = [VERSION + 1, Command::Version as u8, 0];
        let mut parser = Parser::new();
        if let Err(output) = parser.parse(&buffer) {
            assert_eq!(output, Error::NoSyncByte);
        } else {
            assert!(false, "got ok but expected error")
        }
    }

    // #[test]
    // pub fn test_full_fifo() {
    //     let mut fifo_buf = [0u8; 255];
    //     let mut scratch = [0u8; 255];
    //     let mut fifo = Fifo::new(&mut fifo_buf);
    //     let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
    //     fifo.write(&buffer).unwrap();
    //     let parser = Parser::new();
    //     let output = parser.parse_fifo(&mut scratch, &mut fifo).unwrap();
    //     assert_eq!(output, Packet::Echo(b"hi"));
    // }
    //
    // #[test]
    // pub fn test_full_queue() {
    //     const SIZE: usize = MAX_PACKET_SIZE + 1;
    //     let mut scratch = [0u8; MAX_PACKET_SIZE];
    //     let mut queue: Queue<u8, SIZE> = Queue::new();
    //     let (mut producer, mut consumer) = queue.split();
    //     let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
    //     for b in buffer {
    //         producer.enqueue(b).unwrap();
    //     }
    //
    //     let mut parser = Parser::new();
    //     let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
    //     assert_eq!(output, Packet::Echo(b"hi"));
    // }
    //
    // #[test]
    // pub fn test_double_parse_queue() {
    //     const SIZE: usize = MAX_PACKET_SIZE + 1;
    //     let mut scratch = [0u8; MAX_PACKET_SIZE];
    //     let mut queue: Queue<u8, SIZE> = Queue::new();
    //     let (mut producer, mut consumer) = queue.split();
    //     let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
    //     for b in buffer {
    //         producer.enqueue(b).unwrap();
    //     }
    //
    //     let mut parser = Parser::new();
    //     let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
    //     assert_eq!(output, Packet::Echo(b"hi"));
    //
    //     let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'b', b'y', b'e'];
    //     for b in buffer {
    //         producer.enqueue(b).unwrap();
    //     }
    //
    //     let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
    //     assert_eq!(output, Packet::Echo(b"hi"));
    // }
    //
    // #[test]
    // pub fn test_off_center_queue() {
    //     const SIZE: usize = 1024;
    //     let mut scratch = [0u8; MAX_PACKET_SIZE];
    //     let mut queue: Queue<u8, SIZE> = Queue::new();
    //     let (mut producer, mut consumer) = queue.split();
    //     let buffer = [
    //         11,
    //         11,
    //         SYNC_BYTE,
    //         VERSION,
    //         Command::Echo as u8,
    //         2,
    //         b'h',
    //         b'i',
    //     ];
    //     for b in buffer {
    //         producer.enqueue(b).unwrap();
    //     }
    //
    //     let mut parser = Parser::new();
    //     let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
    //     assert_eq!(output, Packet::Echo(b"hi"));
    // }
    //
    // #[test]
    // pub fn test_no_sync_queue() {
    //     const SIZE: usize = 1024;
    //     let mut scratch = [0u8; MAX_PACKET_SIZE];
    //     let mut queue: Queue<u8, SIZE> = Queue::new();
    //     let (mut producer, mut consumer) = queue.split();
    //     let buffer = [VERSION, Command::Echo as u8, 2, b'h', b'i'];
    //     for b in buffer {
    //         producer.enqueue(b).unwrap();
    //     }
    //
    //     let mut parser = Parser::new();
    //     if let Err(output) = parser.parse_queue(&mut scratch, &mut consumer) {
    //         assert_eq!(output, Error::NoSyncByte);
    //     } else {
    //         assert!(false, "Expected Error but got Ok");
    //     }
    // }
}
