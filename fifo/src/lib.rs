#![no_std]

pub struct Fifo<'a> {
    head: usize,
    tail: usize,
    buffer: &'a mut [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    OutOfSpace,
}

impl<'a> Fifo<'a> {
    /// Simple FIFO
    /// Note: This can only write buffer_size - 1 ammount of bytes due to empty check being head ==
    /// tail, a more complex empty system could fix this but this should not need locks
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            head: 0,
            tail: 0,
            buffer,
        }
    }

    /// FIXME: do we want to return the slice of data left so we keep retrying ?
    pub fn write(&mut self, write_buf: &[u8]) -> Result<(), Error> {
        //make sure write wont hit head
        if self.head + write_buf.len() % self.buffer.len() != self.tail {
            //we have enough room, write each byte wrapping if needed
            for i in 0..write_buf.len() {
                self.buffer[self.head] = write_buf[i];
                if self.head + 1 == self.buffer.len() {
                    self.head = 0;
                } else {
                    self.head += 1;
                }
            }
            Ok(())
        } else {
            Err(Error::OutOfSpace)
        }
    }

    pub fn read(&mut self) -> Option<u8> {
        if self.head != self.tail {
            let item = self.buffer[self.tail];
            if self.tail + 1 == self.buffer.len() {
                self.tail = 0;
            } else {
                self.tail += 1;
            }
            Some(item)
        } else {
            None
        }
    }

    /// Reads from fifo until buffer is full or fifo is empty
    /// returns how many items were read
    /// This could read nothing
    pub fn read_to_buffer(&mut self, out_buffer: &mut [u8]) -> usize {
        for i in 0..out_buffer.len() {
            if let Some(item) = self.read() {
                out_buffer[i] = item;
            } else {
                return i;
            }
        }
        return out_buffer.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_write() {
        let mut buffer = [0u8; 20];
        let mut fifo = Fifo::new(&mut buffer);
        fifo.write(&[0xF1]).unwrap();
        assert_eq!(1, fifo.head, "head not written correctly");
        assert_eq!(0, fifo.tail, "tail not written correctly");
        assert_eq!(0xF1, fifo.buffer[0], "Buffer not written correctly");
        let b = fifo.read().unwrap();
        assert_eq!(b, 0xF1);
        assert_eq!(fifo.read(), None);
    }

    /// we can't fill the buffer due to full check being head == tail so full - 1
    #[test]
    fn test_full() {
        let mut buffer = [0u8; 20];
        let write_buffer = [11u8; 19];
        let mut read_buffer = [0u8; 19];

        let mut fifo = Fifo::new(&mut buffer);
        fifo.write(&write_buffer).unwrap();
        assert_eq!(fifo.read_to_buffer(&mut read_buffer), 19);
        assert_eq!(read_buffer, write_buffer);
    }

    #[test]
    fn test_overflow() {
        let mut buffer = [0u8; 20];
        let write_buffer = [11u8; 20];

        let mut fifo = Fifo::new(&mut buffer);
        assert_eq!(fifo.write(&write_buffer), Err(Error::OutOfSpace));
    }

    #[test]
    fn test_wrap_around() {
        let mut buffer = [0u8; 20];
        let mut read_buffer = [0u8; 19];
        let mut fifo = Fifo::new(&mut buffer);
        fifo.write(&[1]).unwrap();

        let b = fifo.read().unwrap();
        assert_eq!(b, 1);
        assert_eq!(fifo.read(), None);

        fifo.write(&[2; 19]).unwrap();
        assert_eq!(fifo.read_to_buffer(&mut read_buffer), 19);
        assert_eq!(read_buffer, [2; 19]);
    }
}
