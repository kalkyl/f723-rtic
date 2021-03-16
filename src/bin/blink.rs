// $ cargo rb blink
#![no_main]
#![no_std]

use f723_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use dwt_systick_monotonic::{
        consts::{U0, U168},
        DwtSystick,
    };
    use embedded_hal::digital::v2::ToggleableOutputPin;
    use rtic::time::duration::Seconds;
    use stm32f7xx_hal::{
        gpio::{gpioa::PA5, Output, PushPull},
        prelude::*,
    };

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = DwtSystick<U168, U0, U0>; // 168 MHz

    #[resources]
    struct Resources {
        led: PA5<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();

        // Set up the system clock.
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(168.mhz()).freeze();

        let gpioa = ctx.device.GPIOA.split();
        let led = gpioa.pa5.into_push_pull_output();

        let mono = DwtSystick::new(
            &mut ctx.core.DCB,
            ctx.core.DWT,
            ctx.core.SYST,
            clocks.hclk().0,
        );

        defmt::info!("Hello world!");
        blink::spawn_after(Seconds(1_u32)).ok();
        (init::LateResources { led }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(resources = [led])]
    fn blink(mut ctx: blink::Context) {
        ctx.resources.led.lock(|l| l.toggle().ok());
        defmt::info!("Blink!");
        blink::spawn_after(Seconds(1_u32)).ok();
    }
}
