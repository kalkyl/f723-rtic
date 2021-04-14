// $ cargo rb exti
#![no_main]
#![no_std]

use f723_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f7xx_hal::device, peripherals = true)]
mod app {
    use embedded_hal::digital::v2::ToggleableOutputPin;
    use stm32f7xx_hal::{
        gpio::{
            gpioa::{PA0, PA5},
            Edge, ExtiPin, Input, Output, PullDown, PushPull,
        },
        prelude::*,
    };
    #[resources]
    struct Resources {
        led: PA5<Output<PushPull>>,
        btn: PA0<Input<PullDown>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        // Enable SYSCFG.
        ctx.device.RCC.apb2enr.write(|w| w.syscfgen().enabled());

        let gpioa = ctx.device.GPIOA.split();
        let led = gpioa.pa5.into_push_pull_output();

        let mut btn = gpioa.pa0.into_pull_down_input();
        btn.make_interrupt_source(&mut ctx.device.SYSCFG);
        btn.trigger_on_edge(&mut ctx.device.EXTI, Edge::RISING);
        btn.enable_interrupt(&mut ctx.device.EXTI);

        // Set up the system clock.
        let rcc = ctx.device.RCC.constrain();
        let _clocks = rcc.cfgr.sysclk(168.mhz()).freeze();

        defmt::info!("Press button!");
        (init::LateResources { btn, led }, init::Monotonics())
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds = EXTI0, resources = [btn, led])]
    fn on_exti(ctx: on_exti::Context) {
        let on_exti::Resources { mut btn, mut led } = ctx.resources;
        btn.lock(|b| b.clear_interrupt_pending_bit());
        led.lock(|l| l.toggle().ok());
        defmt::info!("Button was pressed!");
    }
}
