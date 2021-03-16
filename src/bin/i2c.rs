// $ cargo rb i2c
// Test communication with the onboard audio codec
#![no_main]
#![no_std]

use f723_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
mod app {
    use stm32f7xx_hal::{
        gpio::{
            gpiob::{PB8, PB9},
            Alternate, AF4,
        },
        i2c::{BlockingI2c, Mode},
        pac::I2C1,
        prelude::*,
        rcc::{HSEClock, HSEClockMode},
    };

    const ADDR: u8 = 0x34 >> 1;
    const ID_REG: u32 = 0x0000;
    const VOL_REG: u16 = 0x1C;

    #[resources]
    struct Resources {
        i2c: BlockingI2c<I2C1, PB8<Alternate<AF4>>, PB9<Alternate<AF4>>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        // Set up the system clock.
        let mut rcc = ctx.device.RCC.constrain();
        let clocks = rcc
            .cfgr
            .hse(HSEClock::new(25.mhz(), HSEClockMode::Bypass))
            .use_pll()
            .use_pll48clk()
            .sysclk(216.mhz())
            .freeze();

        // Set up I2C.
        let gpiob = ctx.device.GPIOB.split();
        let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
        let i2c = BlockingI2c::i2c1(
            ctx.device.I2C1,
            (scl, sda),
            Mode::Standard {
                frequency: 400_000.hz(),
            },
            clocks,
            &mut rcc.apb1,
            10000,
        );

        defmt::info!("I2C example!");
        cortex_m::asm::delay(216_000_000);
        (init::LateResources { i2c }, init::Monotonics())
    }

    #[idle(resources=[i2c])]
    fn idle(mut ctx: idle::Context) -> ! {
        loop {
            // Get Device ID
            let mut buf = [0u8; 2];
            ctx.resources.i2c.lock(|i2c| {
                i2c.write_read(ADDR, &ID_REG.to_be_bytes(), &mut buf).ok();
            });
            defmt::info!("Device ID: {:x}", u16::from_be_bytes(buf));
            cortex_m::asm::delay(216_000_000);

            // Get Vol
            let mut buf = [0u8; 2];
            ctx.resources.i2c.lock(|i2c| {
                i2c.write_read(ADDR, &VOL_REG.to_be_bytes(), &mut buf).ok();
            });
            defmt::info!("Vol: {:?}", u16::from_be_bytes(buf));

            // Set Vol
            let mut buf = [0u8; 4];
            let data = 0x11u16;
            buf[..2].copy_from_slice(&VOL_REG.to_be_bytes());
            buf[2..].copy_from_slice(&data.to_be_bytes());
            ctx.resources.i2c.lock(|i2c| {
                i2c.write(ADDR, &buf).ok();
            });
            cortex_m::asm::delay(216_000_000);

            // Get Vol
            let mut buf = [0u8; 2];
            ctx.resources.i2c.lock(|i2c| {
                i2c.write_read(ADDR, &VOL_REG.to_be_bytes(), &mut buf).ok();
            });
            defmt::info!("Vol: {:?}", u16::from_be_bytes(buf));
            cortex_m::asm::delay(216_000_000);
        }
    }
}
