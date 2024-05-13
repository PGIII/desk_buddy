use crate::mem_utils::as_u8_slice;

pub const SYNC_BYTE: u8 = 0xA1;
pub const VERSION: u8 = 0x01;
pub const HEADER_SIZE: usize = core::mem::size_of::<Header>();
pub const MAX_PAYLOAD_SIZE: usize = 0xFF;
pub const MAX_PACKET_SIZE: usize = MAX_PAYLOAD_SIZE + HEADER_SIZE;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Command {
    Ping,
    Echo,
    Version,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(C, packed)]
pub struct Header {
    pub sync: u8, //should be SYNC_BYTE
    pub version: u8,
    pub command: Command,
    pub payload_length: u8, // this gives us max payload of 255, which should be easy for
                            // constrained devices to accommodate
}

#[derive(Debug, PartialEq)]
pub enum Packet<'a> {
    Ping,
    Version,
    Echo(&'a [u8]), //whole payload is the message, dont need to do anything crazy here
}

impl Header {
    pub fn new(command: Command, payload_length: u8) -> Header {
        Header {
            sync: SYNC_BYTE,
            version: VERSION,
            command,
            payload_length,
        }
    }

    pub fn from_packet<'a>(packet: &'a Packet) -> (Header, Option<&'a [u8]>) {
        match packet {
            Packet::Ping => (Header::new(Command::Ping, 0), None),
            Packet::Version => (Header::new(Command::Version, 0), None),
            Packet::Echo(buf) => (Header::new(Command::Echo, buf.len() as u8), Some(buf)),
        }
    }

    /// Returns size a u8 array needs to be to contain the packet
    pub fn packet_size(&self) -> usize {
        HEADER_SIZE + self.payload_length as usize
    }
}
// 1 byte for BW , 250 x 250 pixels, 115200, 4 secs a frame, 15 fps
// 1 bit for BW, 250 x 250 pixels, 115200, .5 sec a frame, 110 fps, 36 fps for 24bit color
impl Packet<'_> {
    pub fn serialize<'a>(&self, buffer: &'a mut [u8]) -> &'a [u8] {
        let (header, payload) = Header::from_packet(self);
        assert!(buffer.len() >= header.packet_size(), "buffer is too small");
        let header_buf = unsafe { as_u8_slice(&header) };
        let mut write_pos = 0;
        for b in header_buf {
            buffer[write_pos] = *b;
            write_pos += 1;
        }
        if let Some(payload) = payload {
            // push buffer, memcpy ????
            for b in payload {
                buffer[write_pos] = *b;
                write_pos += 1;
            }
        }
        &buffer[..write_pos]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    pub fn test_serialize() {
        let packet = Packet::Echo(b"Hi");
        let expected = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'H', b'i'];
        let mut buffer = [0u8; MAX_PACKET_SIZE];

        assert_eq!(expected, packet.serialize(&mut buffer));
    }
}
