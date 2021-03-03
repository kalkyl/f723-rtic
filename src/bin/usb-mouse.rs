// $ cargo rb usb-mouse
#![no_main]
#![no_std]

use f723_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
mod app {
    use stm32f7xx_hal::{
        gpio::{gpioa::PA0, Input, PullDown},
        otg_fs::{UsbBus, UsbBusType, USB},
        prelude::*,
        rcc::{HSEClock, HSEClockMode},
    };
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_hid::{
        descriptor::{generator_prelude::*, MouseReport},
        hid_class::HIDClass,
    };

    #[resources]
    struct Resources {
        btn: PA0<Input<PullDown>>,
        hid: HIDClass<'static, UsbBusType>,
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut EP_MEMORY: [u32; 1024] = [0; 1024];
        static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;

        // Set up the system clock.
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc
            .cfgr
            .hse(HSEClock::new(25.mhz(), HSEClockMode::Bypass))
            .use_pll()
            .use_pll48clk()
            .sysclk(168.mhz())
            .freeze();

        let gpioa = ctx.device.GPIOA.split();
        let btn = gpioa.pa0.into_pull_down_input();

        let usb = USB::new(
            ctx.device.OTG_FS_GLOBAL,
            ctx.device.OTG_FS_DEVICE,
            ctx.device.OTG_FS_PWRCLK,
            (
                gpioa.pa11.into_alternate_af10(),
                gpioa.pa12.into_alternate_af10(),
            ),
            clocks,
        );
        USB_BUS.replace(UsbBus::new(usb, EP_MEMORY));

        let hid = HIDClass::new(USB_BUS.as_ref().unwrap(), MouseReport::desc(), 60);
        let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0xc410, 0x0000))
            .manufacturer("Fake company")
            .product("Mouse")
            .serial_number("TEST")
            .device_class(0)
            .build();

        defmt::info!("Mouse example");
        init::LateResources { btn, hid, usb_dev }
    }

    #[idle(resources=[btn, hid])]
    fn idle(mut ctx: idle::Context) -> ! {
        static mut COUNTER: u8 = 0;
        loop {
            let buttons = match ctx.resources.btn.lock(|b| b.is_high().unwrap()) {
                true => 1,
                false => 0,
            };
            let report = MouseReport {
                x: if *COUNTER < 64 { 3 } else { -3 },
                y: 0,
                buttons,
                wheel: 0,
            };
            ctx.resources.hid.lock(|hid| hid.push_input(&report).ok());
            *COUNTER = (*COUNTER + 1) % 128;
            cortex_m::asm::delay(2_000_000);
        }
    }

    #[task(binds=OTG_FS, resources = [hid, usb_dev])]
    fn on_usb(mut ctx: on_usb::Context) {
        let mut usb_dev = ctx.resources.usb_dev;
        ctx.resources.hid.lock(|hid| {
            if !usb_dev.lock(|u| u.poll(&mut [hid])) {
                return;
            }
        });
    }
}
