#![allow(static_mut_refs)]
#![no_std]
#![no_main]

extern crate alloc;

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts, dma,
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0},
    pio::InterruptHandler,
};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use embedded_alloc::LlffHeap as Heap;

use cyw43::SpiBus;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

#[global_allocator]
static ALLOCATOR: Heap = Heap::empty();

const HEAP_SIZE: usize = 64 * 1024;

static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

fn init_heap() {
    unsafe {
        ALLOCATOR.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
    }
}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
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
    init_heap();

    let p = embassy_rp::init(Default::default());

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);

    let mut pio = embassy_rp::pio::Pio::new(p.PIO0, Irqs);

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

    let fw = unsafe {
        core::mem::transmute::<&[u8], &cyw43::Aligned<cyw43::A4, [u8]>>(
            cyw43_firmware::CYW43_43439A0,
        )
    };

    let clm = unsafe {
        core::mem::transmute::<&[u8], &cyw43::Aligned<cyw43::A4, [u8]>>(
            cyw43_firmware::CYW43_43439A0_CLM,
        )
    };

    let state = STATE.init(cyw43::State::new());

    let (_, _, runner) = cyw43::new(state, pwr, spi, fw, clm).await;

    spawner.spawn(wifi_task(runner).unwrap());

    let mut i = 0;

    loop {
        Timer::after(Duration::from_secs(1)).await;
        info!("alive {}", i);
        i += 1;
    }
}
