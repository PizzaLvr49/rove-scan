#![no_std]
#![no_main]

extern crate alloc;

use core::mem::MaybeUninit;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::PIO0,
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
static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

fn init_heap() {
    unsafe {
        ALLOCATOR.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE);
    }
}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
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
        p.DMA_CH0,
    );

    let fw = cyw43_firmware::FIRMWARE;
    let clm = cyw43_firmware::CLM;

    let state = STATE.init(cyw43::State::new());

    let (_net_device, _control, runner) = cyw43::new(state, pwr, spi, fw, clm).await;

    spawner.spawn(wifi_task(runner).unwrap());

    let mut i = 0u32;

    loop {
        Timer::after(Duration::from_secs(1)).await;
        info!("alive {}", i);
        i += 1;
    }
}
