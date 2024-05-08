#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

pub struct Fifo<'a, T> {
    head: AtomicUsize,
    tail: AtomicUsize,
    buffer: &'a mut [T],
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    OutOfSpace,
}

impl<'a, T> Fifo<'a, T>
where
    T: Copy,
{
    /// Simple FIFO
    /// Note: This can only write buffer_size - 1 amount of bytes due to empty check being head ==
    /// tail, a more complex empty system could fix this but this should not need locks
    pub fn new(buffer: &'a mut [T]) -> Self {
        Self {
            head: 0.into(),
            tail: 0.into(),
            buffer,
        }
    }

    /// FIXME: do we want to return the slice of data left so we keep retrying ?
    pub fn write(&mut self, write_buf: &[T]) -> Result<(), Error> {
        //make sure write wont hit head
        if self.head.load(Ordering::SeqCst) + write_buf.len() % self.buffer.len()
            != self.tail.load(Ordering::SeqCst)
        {
            //we have enough room, write each byte wrapping if needed
            for i in 0..write_buf.len() {
                let head = self.head.load(Ordering::SeqCst);
                self.buffer[head] = write_buf[i];
                if head + 1 == self.buffer.len() {
                    self.head.store(0, Ordering::SeqCst);
                } else {
                    self.head.fetch_add(1, Ordering::SeqCst);
                }
            }
            Ok(())
        } else {
            Err(Error::OutOfSpace)
        }
    }

    /// Read next item from fifo and advance tail
    pub fn read(&mut self) -> Option<T> {
        let tail = self.tail.load(Ordering::SeqCst);
        if self.head.load(Ordering::SeqCst) != tail {
            let item = self.buffer[tail];
            if tail + 1 == self.buffer.len() {
                self.tail.store(0, Ordering::SeqCst);
            } else {
                self.tail.fetch_add(1, Ordering::SeqCst);
            }
            Some(item)
        } else {
            None
        }
    }

    /// Read the next item but don't advance tail
    pub fn peek(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::SeqCst);
        if self.head.load(Ordering::SeqCst) != tail {
            Some(self.buffer[tail])
        } else {
            None
        }
    }

    /// Reads from fifo until buffer is full or fifo is empty
    /// returns how many items were read
    /// This could read nothing
    pub fn read_to_buffer(&mut self, out_buffer: &mut [T]) -> usize {
        for i in 0..out_buffer.len() {
            if let Some(item) = self.read() {
                out_buffer[i] = item;
            } else {
                return i;
            }
        }
        return out_buffer.len();
    }

    /// returns how many items are currently in fifo
    pub fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::SeqCst);
        let head = self.head.load(Ordering::SeqCst);
        if head == tail {
            0
        } else if head > tail {
            head - tail
        } else {
            let to_end = self.buffer.len() - tail;
            let to_head = head;
            to_end + to_head
        }
    }

    /// Returns max count of items fifo can hold
    pub fn size(&self) -> usize {
        self.buffer.len() - 1
    }

    /// Returns how many slots remain
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_write() {
        let mut buffer = [0u8; 20];
        let mut fifo = Fifo::new(&mut buffer);
        assert_eq!(fifo.len(), 0);
        fifo.write(&[0xF1]).unwrap();
        assert_eq!(fifo.len(), 1);
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
        assert_eq!(fifo.len(), 19);
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
        assert_eq!(fifo.len(), 19);
        assert_eq!(fifo.read_to_buffer(&mut read_buffer), 19);
        assert_eq!(read_buffer, [2; 19]);
    }
}
