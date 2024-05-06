pub const SYNC_BYTE: u8 = 0xA1;
pub const VERSION: u8 = 0x01;

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
    pub payload_length: u16,
}

#[derive(Debug, PartialEq)]
pub enum Packet<'a> {
    Ping,
    Version,
    Echo(&'a [u8]), //whole payload is the message, dont need to do anything crazy here
}
