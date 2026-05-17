#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    config::Config,
    dma,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0},
    pio::{self, Pio},
};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use cyw43::{SpiBus, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

static STATE: StaticCell<cyw43::State> = StaticCell::new();

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

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

    let (_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;

    spawner.spawn(unwrap!(wifi_task(runner)));

    control
        .init(include_bytes!("../firmware/43439A0_clm.bin"))
        .await;

    let mut i = 0;

    loop {
        Timer::after(Duration::from_secs(1)).await;
        info!("alive {}", i);
        i += 1;
    }
}
