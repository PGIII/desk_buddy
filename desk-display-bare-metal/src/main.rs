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
use embassy_time::{Delay, Timer};
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::iso_8859_5::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io::Write as EIOWrite;
use esp_backtrace as _;
use esp_hal::gpio::NO_PIN;
use esp_hal::{
    clock::ClockControl,
    dma::*,
    embassy,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    rmt::Rmt,
    spi::{
        master::{prelude::*, Spi},
        SpiMode,
    },
    timer::TimerGroup,
    usb_serial_jtag::{UsbSerialJtag, UsbSerialJtagRx, UsbSerialJtagTx},
    Async,
};
use esp_hal::{dma_descriptors, spi, FlashSafeDma};
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use esp_println::println;
use heapless::spsc::{Consumer, Producer, Queue};
use log::info;
use smart_leds::hsv::Hsv;
use smart_leds::{brightness, gamma, hsv::hsv2rgb, SmartLedsWrite};
use ssd1680::async_driver::Ssd1680Async;
use ssd1680::driver::Ssd1680;
use ssd1680::graphics::{Display, Display2in13, DisplayRotation};

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

    println!("Building display");
    let spi = peripherals.SPI2;
    let rst = io.pins.gpio13.into_push_pull_output();
    let dc = io.pins.gpio12.into_push_pull_output();
    let busy = io.pins.gpio14.into_pull_up_input(); //FIXME: what should this be ?
    let sclk = io.pins.gpio10;
    let mosi = io.pins.gpio9;
    let cs = io.pins.gpio11.into_push_pull_output();
    let dma = Dma::new(peripherals.DMA);

    //TODO: why is dma this way?
    #[cfg(any(feature = "esp32", feature = "esp32s2"))]
    let dma_channel = dma.spi2channel;
    #[cfg(not(any(feature = "esp32", feature = "esp32s2")))]
    let dma_channel = dma.channel0;

    //TODO: are these buffers?
    let (mut descriptors, mut rx_descriptors) = dma_descriptors!(3200);
    let spi = Spi::new(spi, 50_000.kHz(), SpiMode::Mode0, &clocks).with_pins(
        Some(sclk),
        Some(mosi),
        NO_PIN,
        NO_PIN,
    );
    // .with_dma(dma_channel.configure_for_async(
    //     false,
    //     &mut descriptors,
    //     &mut rx_descriptors,
    //     DmaPriority::Priority0,
    // ));
    //FIXME: investigate this if it's needed or not
    //let spi = FlashSafeDma::<_, 6000>::new(spi);
    let spi_device = ExclusiveDevice::new(spi, cs, Delay).unwrap();
    let disp_interface = display_interface_spi::SPIInterface::new(spi_device, dc);
    let mut delay = Delay;
    let mut ssd1680 = Ssd1680::new(disp_interface, busy, rst, &mut delay).unwrap();
    ssd1680.clear_bw_frame().unwrap();
    let mut display_bw = Display2in13::bw();
    display_bw.set_rotation(DisplayRotation::Rotate90);
    println!("drawing display");
    // background fill
    display_bw
        .fill_solid(&display_bw.bounding_box(), BinaryColor::On)
        .unwrap();

    Text::new(
        "hello",
        Point::new(10, 10),
        MonoTextStyle::new(&FONT_6X9, BinaryColor::Off),
    )
    .draw(&mut display_bw)
    .unwrap();
    println!("updating display");
    ssd1680.update_bw_frame(display_bw.buffer()).unwrap();
    ssd1680.display_frame(&mut delay).unwrap();

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
