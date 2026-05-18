#![no_std]
#![no_main]

use core::net::Ipv4Addr;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_net::{
    self, Config as NetConfig, DhcpConfig, IpAddress, StackResources,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_rp::{
    bind_interrupts,
    clocks::ClockConfig,
    config::Config,
    dma,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0},
    pio::{self, Pio},
};
use embassy_time::{Duration, Timer};

use static_cell::StaticCell;

use cyw43::{JoinOptions, NetDriver, SpiBus, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

static STATE: StaticCell<cyw43::State> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let config = ClockConfig::system_freq(250_000_000).expect("Clock Init Failed");

    let p = embassy_rp::init(Config::new(config));

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);

    let mut pio = Pio::new(p.PIO0, Irqs);

    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    let fw = aligned_bytes!("../firmware/43439A0.bin");

    let nvram = aligned_bytes!("../firmware/43439A0_nvram.bin");

    let state = STATE.init(cyw43::State::new());

    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;

    spawner.spawn(unwrap!(wifi_task(runner)));

    control
        .init(include_bytes!("../firmware/43439A0_clm.bin"))
        .await;

    control
        .set_power_management(cyw43::PowerManagementMode::None)
        .await;

    control
        .join(SSID, JoinOptions::new(PASSWORD.as_bytes()))
        .await
        .unwrap();

    info!("WiFi connected");

    let config = NetConfig::dhcpv4(DhcpConfig::default());

    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        0x1234,
    );

    spawner.spawn(unwrap!(net_task(runner)));

    Timer::after(Duration::from_secs(3)).await;

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];

    let mut rx = [0u8; 256];
    let mut tx = [0u8; 256];

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx, &mut tx_meta, &mut tx);
    socket.bind(0).unwrap();

    let msg = b"hello world";

    loop {
        Timer::after(Duration::from_secs(1)).await;

        let result = socket
            .send_to(msg, (IpAddress::Ipv4(Ipv4Addr::new(192, 168, 1, 50)), 9000))
            .await;

        if let Err(err) = result {
            warn!("net error: {}", err);
        }

        info!("sent");
    }
}
