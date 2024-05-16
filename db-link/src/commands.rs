use crate::mem_utils::as_u8_slice;

pub const SYNC_BYTE: u8 = 0xA1;
pub const VERSION: u8 = 0x01;
pub const HEADER_SIZE: usize = core::mem::size_of::<Header>();
pub const MAX_PAYLOAD_SIZE: usize = 0xFF;
pub const MAX_PACKET_SIZE: usize = MAX_PAYLOAD_SIZE + HEADER_SIZE;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Command {
    Echo,
    GetParam,
    SetParam,
    GetParamList,
    Response,
    Error,
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

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(C, packed)]
pub struct ResponsePayload {
    pub command: Command,
    pub msg: [u8; MAX_PAYLOAD_SIZE - 1],
}

pub type PayloadBuf = heapless::Vec<u8, MAX_PAYLOAD_SIZE>;
#[derive(Debug, PartialEq, Clone)]
pub enum Packet {
    Echo(PayloadBuf), //whole payload is the message, dont need to do anything crazy here
    GetParamList,
    GetParam(PayloadBuf),
    SetParam(PayloadBuf),
    Response(PayloadBuf),
    Error(PayloadBuf),
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

    pub fn from_packet(packet: Packet) -> (Header, Option<PayloadBuf>) {
        match packet {
            Packet::Echo(buf) => (Header::new(Command::Echo, buf.len() as u8), Some(buf)),
            Packet::GetParam(buf) => (Header::new(Command::GetParam, buf.len() as u8), Some(buf)),
            Packet::SetParam(buf) => (Header::new(Command::SetParam, buf.len() as u8), Some(buf)),
            Packet::GetParamList => (Header::new(Command::GetParamList, 0), None),
            Packet::Response(response) => (
                Header::new(Command::Response, response.len() as u8),
                Some(response),
            ),
            Packet::Error(response) => (
                Header::new(Command::Error, response.len() as u8),
                Some(response),
            ),
        }
    }

    /// Returns size a u8 array needs to be to contain the packet
    pub fn packet_size(&self) -> usize {
        HEADER_SIZE + self.payload_length as usize
    }
}
// 1 byte for BW , 250 x 250 pixels, 115200, 4 secs a frame, 15 fps
// 1 bit for BW, 250 x 250 pixels, 115200, .5 sec a frame, 110 fps, 36 fps for 24bit color
impl Packet {
    pub fn serialize(self) -> heapless::Vec<u8, MAX_PACKET_SIZE> {
        let mut vec = heapless::Vec::<u8, MAX_PACKET_SIZE>::new();
        let (header, payload) = Header::from_packet(self);
        let header_buf = unsafe { as_u8_slice(&header) };
        for b in header_buf {
            //unwrap should be fine here since we're controlling all the sizes
            //if we run out of space that's a big error
            vec.push(*b).unwrap();
        }
        if let Some(payload) = payload {
            // push buffer, memcpy ????
            vec.extend(payload);
        }
        vec
    }

    #[cfg(feature = "std")]
    pub fn serialize_vec(self) -> std::vec::Vec<u8> {
        use std::vec;
        let mut vec = vec![];
        let (header, payload) = Header::from_packet(self);
        let header_buf = unsafe { as_u8_slice(&header) };
        vec.extend_from_slice(header_buf);
        if let Some(payload) = payload {
            vec.extend(payload);
        }
        vec
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    pub fn test_serialize() {
        let packet = Packet::Echo(PayloadBuf::from_slice(b"Hi").unwrap());
        let expected = [SYNC_BYTE, VERSION, Command::Echo as u8, 2, b'H', b'i'];
        let mut buffer = [0u8; MAX_PACKET_SIZE];

        assert_eq!(expected, packet.serialize());
    }
}
