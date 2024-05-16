#![no_std]
#![no_main]

use core::fmt::Write;
use core::ptr::addr_of_mut;

use db_link::commands::{Command, MAX_PACKET_SIZE, MAX_PAYLOAD_SIZE};
use db_link::{
    commands::{Packet, PayloadBuf, ResponsePayload},
    parser::Parser,
};
use embassy_executor::Spawner;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embedded_io::Write as EIOWrite;
use esp_backtrace as _;
use esp_hal::rmt::Rmt;
use esp_hal::{
    clock::ClockControl,
    embassy,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagRx, UsbSerialJtagTx},
    Async,
};
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use heapless::spsc::{Consumer, Producer, Queue};
use log::info;
use smart_leds::hsv::Hsv;
use smart_leds::{brightness, gamma, hsv::hsv2rgb, SmartLedsWrite};

const QUEUE_SIZE: usize = 4096;
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Handle packets, return vector should be sent back
fn handle_packet(packet: Packet) -> heapless::Vec<u8, MAX_PACKET_SIZE> {
    match packet {
        Packet::Echo(_) => packet.serialize(),
        Packet::GetParam(param) => match param.as_slice() {
            b"VERSION" => {
                Packet::Response(PayloadBuf::from_slice(VERSION.as_bytes()).unwrap()).serialize()
            }
            _ => {
                let mut buf = heapless::Vec::<u8, MAX_PAYLOAD_SIZE>::new();
                _ = buf.write_str("unknown param");
                Packet::Error(buf).serialize()
            }
        },
        _ => {
            let mut buf = heapless::Vec::<u8, MAX_PAYLOAD_SIZE>::new();
            _ = buf.write_str("unknown command");
            Packet::Error(buf).serialize()
        }
    }
}

#[embassy_executor::task]
async fn writer(
    mut tx: UsbSerialJtagTx<'static, Async>,
    signal: &'static Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, usize>,
    mut fifo: Consumer<'static, u8, QUEUE_SIZE>,
) {
    let mut parser = Parser::new();

    loop {
        // info!("P Waiting");
        let _len = signal.wait().await;
        signal.reset();
        // info!("P Got {len}bytes");
        while let Some(byte) = fifo.dequeue() {
            match parser.parse(&[byte]) {
                Ok(packet) => {
                    //TODO: I asumme this should only write the len of the vec but should check
                    //this
                    let buf = handle_packet(packet);
                    tx.write_all(&buf).unwrap();
                    //embedded_io_async::Write::flush(&mut tx).await.unwrap();
                    // info!("P Wrote Packet");
                }
                Err(db_link::parser::Error::InvalidVersion) => {
                    log::error!("P Received invalid version");
                }
                Err(_) => {}
            }
        }
    }
}

#[embassy_executor::task]
async fn reader(
    mut rx: UsbSerialJtagRx<'static, Async>,
    signal: &'static Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, usize>,
    mut fifo: Producer<'static, u8, QUEUE_SIZE>,
) {
    let mut rbuf = [0u8; 512];
    loop {
        // info!("R reading");
        let r = embedded_io_async::Read::read(&mut rx, &mut rbuf).await;
        match r {
            Ok(len) => {
                for b in &rbuf[..len] {
                    while let Err(_) = fifo.enqueue(*b) {
                        //fifo full, try to give parser time to process
                        // info!("R fifo full");
                        Timer::after_millis(20).await;
                    }
                }
                // info!("R signaling");
                signal.signal(len);
            }
            Err(e) => esp_println::println!("RX Error: {:?}", e),
        }
    }
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let (tx, rx) = UsbSerialJtag::new_async(peripherals.USB_DEVICE).split();
    esp_println::logger::init_logger_from_env();

    let fifo: &'static mut Queue<u8, QUEUE_SIZE> = {
        static mut Q: Queue<u8, QUEUE_SIZE> = Queue::new();
        unsafe { &mut *addr_of_mut!(Q) }
    };
    static DATA_SIGNAL: Signal<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, usize> =
        Signal::new();

    let (producer, consumer) = fifo.split();

    spawner.spawn(reader(rx, &DATA_SIGNAL, producer)).unwrap();
    spawner.spawn(writer(tx, &DATA_SIGNAL, consumer)).unwrap();

    let rmt = Rmt::new(peripherals.RMT, 80.MHz(), &clocks, None).unwrap();
    let rmt_buffer = smartLedBuffer!(1);
    let mut led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio38, rmt_buffer, &clocks);

    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };
    let mut data;
    loop {
        // Iterate over the rainbow!
        for hue in 0..=255 {
            color.hue = hue;
            // Convert from the HSV color space (where we can easily transition from one
            // color to the other) to the RGB color space that we can then send to the LED
            data = [hsv2rgb(color)];
            // When sending to the LED, we do a gamma correction first (see smart_leds
            // documentation for details) and then limit the brightness to 10 out of 255 so
            // that the output it's not too bright.
            led.write(brightness(gamma(data.iter().cloned()), 10))
                .unwrap();
            Timer::after_millis(20).await;
        }
    }
}
