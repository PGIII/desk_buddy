// originally addded this for memmem but not sure we need it, maybe its faster though
use crate::commands::{
    Command, Header, Packet, HEADER_SIZE, MAX_PACKET_SIZE, MAX_PAYLOAD_SIZE, SYNC_BYTE, VERSION,
};
use fifo::Fifo;
use heapless::spsc::Consumer;
use memchr::memchr;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidVersion,
    NoSyncByte,
    InCompletePayload,
    InCompleteHeader,
}

pub enum Status {
    WaitingForSync,
    WaitingForHeader,
    WaitingForPayload(Command, usize), //command, and payload length
}

pub struct Parser {
    status: Status,
    buffer_pos: usize,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            status: Status::WaitingForSync,
            buffer_pos: 0,
        }
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
        payload_buffer: &'a mut [u8; MAX_PAYLOAD_SIZE],
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
                                let bytes_read = fifo.read_to_buffer(payload_buffer);
                                return Ok(parse_command(
                                    header.command,
                                    &payload_buffer[..bytes_read],
                                ));
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

    pub fn parse_queue<'a, const N: usize>(
        &mut self,
        packet_buffer: &'a mut [u8; MAX_PACKET_SIZE],
        queue: &mut Consumer<'a, u8, N>,
    ) -> Result<Packet<'a>, Error> {
        while let Some(b) = queue.dequeue() {
            packet_buffer[self.buffer_pos] = b;
            self.buffer_pos += 1;
            match self.status {
                Status::WaitingForSync => {
                    if packet_buffer[0] == SYNC_BYTE {
                        self.status = Status::WaitingForHeader;
                    } else {
                        self.buffer_pos = 0;
                    }
                }
                Status::WaitingForHeader => {
                    if self.buffer_pos == HEADER_SIZE {
                        let header = buffer_to_header(&packet_buffer[..HEADER_SIZE]);
                        self.status = Status::WaitingForPayload(
                            header.command,
                            header.payload_length as usize,
                        )
                    }
                }
                Status::WaitingForPayload(command, payload_len) => {
                    if self.buffer_pos == HEADER_SIZE + payload_len {
                        self.buffer_pos = 0;
                        return Ok(parse_command(
                            command,
                            &packet_buffer[HEADER_SIZE..HEADER_SIZE + payload_len],
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
    use heapless::spsc::Queue;
    use paste;

    #[test]
    pub fn test_parse_buffer() {
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
        let parser = Parser::new();
        let output = parser.parse_buffer(&buffer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }

    #[test]
    pub fn test_full_fifo() {
        let mut fifo_buf = [0u8; 255];
        let mut scratch = [0u8; 255];
        let mut fifo = Fifo::new(&mut fifo_buf);
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
        fifo.write(&buffer).unwrap();
        let parser = Parser::new();
        let output = parser.parse_fifo(&mut scratch, &mut fifo).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }

    #[test]
    pub fn test_full_queue() {
        const SIZE: usize = MAX_PACKET_SIZE + 1;
        let mut scratch = [0u8; MAX_PACKET_SIZE];
        let mut queue: Queue<u8, SIZE> = Queue::new();
        let (mut producer, mut consumer) = queue.split();
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
        for b in buffer {
            producer.enqueue(b).unwrap();
        }

        let mut parser = Parser::new();
        let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }

    #[test]
    pub fn test_double_parse_queue() {
        const SIZE: usize = MAX_PACKET_SIZE + 1;
        let mut scratch = [0u8; MAX_PACKET_SIZE];
        let mut queue: Queue<u8, SIZE> = Queue::new();
        let (mut producer, mut consumer) = queue.split();
        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'h', b'i'];
        for b in buffer {
            producer.enqueue(b).unwrap();
        }

        let mut parser = Parser::new();
        let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));

        let buffer = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'b', b'y', b'e'];
        for b in buffer {
            producer.enqueue(b).unwrap();
        }

        let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }

    #[test]
    pub fn test_off_center_queue() {
        const SIZE: usize = 1024;
        let mut scratch = [0u8; MAX_PACKET_SIZE];
        let mut queue: Queue<u8, SIZE> = Queue::new();
        let (mut producer, mut consumer) = queue.split();
        let buffer = [
            11,
            11,
            SYNC_BYTE,
            VERSION,
            Command::Echo as u8,
            2,
            b'h',
            b'i',
        ];
        for b in buffer {
            producer.enqueue(b).unwrap();
        }

        let mut parser = Parser::new();
        let output = parser.parse_queue(&mut scratch, &mut consumer).unwrap();
        assert_eq!(output, Packet::Echo(b"hi"));
    }

    #[test]
    pub fn test_no_sync_queue() {
        const SIZE: usize = 1024;
        let mut scratch = [0u8; MAX_PACKET_SIZE];
        let mut queue: Queue<u8, SIZE> = Queue::new();
        let (mut producer, mut consumer) = queue.split();
        let buffer = [VERSION, Command::Echo as u8, 2, b'h', b'i'];
        for b in buffer {
            producer.enqueue(b).unwrap();
        }

        let mut parser = Parser::new();
        if let Err(output) = parser.parse_queue(&mut scratch, &mut consumer) {
            assert_eq!(output, Error::NoSyncByte);
        } else {
            assert!(false, "Expected Error but got Ok");
        }
    }
}
