// $ cargo rb usb-serial
#![no_main]
#![no_std]

use f723_rtic as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
mod app {
    use stm32f7xx_hal::{
        otg_fs::{UsbBus, UsbBusType, USB},
        prelude::*,
        rcc::{HSEClock, HSEClockMode},
    };
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_serial::SerialPort;

    #[resources]
    struct Resources {
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
        serial: SerialPort<'static, UsbBus<USB>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (init::LateResources, init::Monotonics) {
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

        let serial = SerialPort::new(USB_BUS.as_ref().unwrap());
        let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("Fake company")
            .product("Serial port")
            .serial_number("TEST")
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

        defmt::info!("Send me a string!");
        (init::LateResources { serial, usb_dev }, init::Monotonics())
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds=OTG_FS, resources = [serial, usb_dev])]
    fn on_usb(mut ctx: on_usb::Context) {
        let mut usb_dev = ctx.resources.usb_dev;
        ctx.resources.serial.lock(|serial| {
            if !usb_dev.lock(|u| u.poll(&mut [serial])) {
                return;
            }
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    defmt::info!("Received: {:?}", core::str::from_utf8(&buf[..]).unwrap());
                    // Echo back in upper case
                    for c in buf[0..count].iter_mut() {
                        if 0x61 <= *c && *c <= 0x7a {
                            *c &= !0x20;
                        }
                    }
                    let mut write_offset = 0;
                    while write_offset < count {
                        match serial.write(&buf[write_offset..count]) {
                            Ok(len) if len > 0 => {
                                write_offset += len;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        });
    }
}
