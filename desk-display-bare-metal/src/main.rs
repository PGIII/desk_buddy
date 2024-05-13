#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use db_link::{commands::Packet, parser::Parser};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use embassy_time::Timer;
use embedded_io::Write;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    embassy,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagRx, UsbSerialJtagTx},
    Async,
};
use fifo::Fifo;
use heapless::spsc::{Consumer, Producer, Queue};
use static_cell::StaticCell;

const MAX_BUFFER_SIZE: usize = 512;

#[embassy_executor::task]
async fn writer(mut tx: UsbSerialJtagTx<'static, Async>, mut fifo: Consumer<'static, u8, 1024>) {
    use core::fmt::Write;
    embedded_io_async::Write::write_all(
        &mut tx,
        b"Hello async USB Serial JTAG. Type something.\r\n",
    )
    .await
    .unwrap();
    let mut parser = Parser::new();

    loop {
        match fifo.dequeue() {
            Some(byte) => {
                //log::info!("Got {}", byte);
                match parser.parse(&[byte]) {
                    Ok(packet) => match packet {
                        Packet::Echo(_) => {
                            let mut buf = [0u8; MAX_BUFFER_SIZE];
                            tx.write_all(packet.serialize(&mut buf)).unwrap();
                            embedded_io_async::Write::flush(&mut tx).await.unwrap();
                        }
                        _ => todo!(),
                    },
                    Err(db_link::parser::Error::InvalidVersion) => {
                        log::error!("Received invalid version");
                    }
                    Err(_) => {}
                }
                // write!(&mut tx, "-- received '{}' --\r\n", byte).unwrap();
                // embedded_io_async::Write::flush(&mut tx).await.unwrap();
            }
            None => {
                Timer::after_secs(1).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn reader(mut rx: UsbSerialJtagRx<'static, Async>, mut fifo: Producer<'static, u8, 1024>) {
    let mut rbuf = [0u8; MAX_BUFFER_SIZE];
    loop {
        let r = embedded_io_async::Read::read(&mut rx, &mut rbuf).await;
        match r {
            Ok(len) => {
                for b in &rbuf[..len] {
                    fifo.enqueue(*b).unwrap();
                }
            }
            Err(e) => esp_println::println!("RX Error: {:?}", e),
        }
    }
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let (tx, rx) = UsbSerialJtag::new_async(peripherals.USB_DEVICE).split();
    esp_println::logger::init_logger_from_env();

    //can static cell fix this ?
    let fifo: &'static mut Queue<u8, 1024> = {
        static mut Q: Queue<u8, 1024> = Queue::new();
        unsafe { &mut Q }
    };

    let (producer, consumer) = fifo.split();

    spawner.spawn(reader(rx, producer)).unwrap();
    spawner.spawn(writer(tx, consumer)).unwrap()
}
