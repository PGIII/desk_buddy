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

// 1 byte for BW , 250 x 250 pixels, 115200, 4 secs a frame, 15 fps
// 1 bit for BW, 250 x 250 pixels, 115200, .5 sec a frame, 110 fps, 36 fps for 24bit color
