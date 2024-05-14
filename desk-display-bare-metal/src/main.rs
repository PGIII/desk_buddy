#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use db_link::commands::MAX_PAYLOAD_SIZE;
use db_link::{commands::Packet, parser::Parser};
use embassy_executor::Spawner;
use embassy_sync::signal::Signal;
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
use heapless::spsc::{Consumer, Producer, Queue};

const MAX_BUFFER_SIZE: usize = 512;

/// Handle packets, return vector should be sent back
fn handle_packet(packet: Packet) -> heapless::Vec<u8, MAX_PAYLOAD_SIZE> {
    let mut vec = heapless::Vec::<u8, MAX_PAYLOAD_SIZE>::new();
    match packet {
        Packet::Echo(_) => {
            //FIXME: holy allocations batman
            let mut buf = [0u8; MAX_BUFFER_SIZE];
            let response = packet.serialize(&mut buf);
            for b in response {
                if vec.push(*b).is_err() {
                    //This should't be able to happen, if it does probably and error in the
                    //protocol
                    //or *gasp* a memory error
                    panic!("Ran out of space writing packet to vec")
                }
            }
        }
        _ => todo!(),
    }
    vec
}

#[embassy_executor::task]
async fn writer(
    mut tx: UsbSerialJtagTx<'static, Async>,
    signal: &'static Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, usize>,
    mut fifo: Consumer<'static, u8, 1024>,
) {
    let mut parser = Parser::new();
    loop {
        match fifo.dequeue() {
            Some(byte) => {
                match parser.parse(&[byte]) {
                    Ok(packet) => {
                        //TODO: I asumme this should only write the len of the vec but should check
                        //this
                        let buf = handle_packet(packet);
                        tx.write_all(&buf).unwrap();
                        embedded_io_async::Write::flush(&mut tx).await.unwrap();
                    }
                    Err(db_link::parser::Error::InvalidVersion) => {
                        log::error!("Received invalid version");
                    }
                    Err(_) => {}
                }
            }
            None => _ = signal.wait().await,
        }
    }
}

#[embassy_executor::task]
async fn reader(
    mut rx: UsbSerialJtagRx<'static, Async>,
    signal: &'static Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, usize>,
    mut fifo: Producer<'static, u8, 1024>,
) {
    let mut rbuf = [0u8; MAX_BUFFER_SIZE];
    loop {
        let r = embedded_io_async::Read::read(&mut rx, &mut rbuf).await;
        match r {
            Ok(len) => {
                for b in &rbuf[..len] {
                    fifo.enqueue(*b).unwrap();
                }
                signal.signal(len);
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

    let fifo: &'static mut Queue<u8, 1024> = {
        static mut Q: Queue<u8, 1024> = Queue::new();
        unsafe { &mut *addr_of_mut!(Q) }
    };
    static DATA_SIGNAL: Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, usize> =
        Signal::new();

    let (producer, consumer) = fifo.split();

    spawner.spawn(reader(rx, &DATA_SIGNAL, producer)).unwrap();
    spawner.spawn(writer(tx, &DATA_SIGNAL, consumer)).unwrap()
}
