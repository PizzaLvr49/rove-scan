#![no_std]
#![no_main]

use core::net::Ipv4Addr;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_net::{
    Config as NetConfig, DhcpConfig, IpAddress, StackResources,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_rp::{
    bind_interrupts,
    clocks::ClockConfig,
    config::Config,
    dma,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0, TRNG},
    pio::{self, Pio},
    trng::{self, Trng},
};
use embassy_time::{Duration, Ticker, Timer, with_timeout};

use rand::RngCore;
use static_cell::StaticCell;

use cyw43::{JoinOptions, NetDriver, SpiBus, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    TRNG_IRQ  => trng::InterruptHandler<TRNG>;
});

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

const TARGET_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 50);
const TARGET_PORT: u16 = 9000;
const LOCAL_PORT: u16 = 9001;

const MAX_SOCKETS: usize = 1;

static STATE: StaticCell<cyw43::State> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<MAX_SOCKETS>> = StaticCell::new();

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
    let clock_config = unwrap!(ClockConfig::system_freq(250_000_000), "Clock init failed");

    let p = embassy_rp::init(Config::new(clock_config));

    let mut rng = Trng::new(p.TRNG, Irqs, trng::Config::default());

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

    loop {
        match control
            .join(SSID, JoinOptions::new(PASSWORD.as_bytes()))
            .await
        {
            Ok(()) => {
                info!("WiFi connected");
                break;
            }
            Err(err) => {
                warn!("WiFi join failed: {}, retrying in 5s...", err);
                Timer::after(Duration::from_secs(5)).await;
            }
        }
    }

    let net_config = NetConfig::dhcpv4(DhcpConfig::default());

    let (stack, runner) = embassy_net::new(
        net_device,
        net_config,
        RESOURCES.init(StackResources::new()),
        rng.next_u64(),
    );

    spawner.spawn(unwrap!(net_task(runner)));

    info!("Waiting for DHCP...");
    if with_timeout(Duration::from_secs(5), stack.wait_config_up())
        .await
        .is_err()
    {
        error!("DHCP Timeout");
    }

    let ip = unwrap!(stack.config_v4(), "No IPv4 config after DHCP").address;
    info!("IP address: {}", ip);

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx = [0u8; 256];
    let mut tx = [0u8; 256];

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx, &mut tx_meta, &mut tx);

    unwrap!(socket.bind(LOCAL_PORT), "UDP bind failed");

    let msg = b"hello world";

    let mut ticker = Ticker::every(Duration::from_secs(1));

    loop {
        ticker.next().await;

        match socket
            .send_to(msg, (IpAddress::Ipv4(TARGET_IP), TARGET_PORT))
            .await
        {
            Ok(()) => info!("Sent to {}:{}", TARGET_IP, TARGET_PORT),
            Err(err) => warn!("Send failed: {}", err),
        }
    }
}
