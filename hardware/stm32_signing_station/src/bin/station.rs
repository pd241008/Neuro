#![no_std]
#![no_main]

use cortex_m_rt::entry;
use stm32f1xx_hal as hal;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::serial::{Config, Serial};

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

/// Required by cortex-m-rt's Reset handler.
#[no_mangle]
pub unsafe extern "C" fn __pre_init() {}

#[entry]
fn main() -> ! {
    let dp = hal::pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.constrain();
    let mut flash = dp.FLASH.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut gpioa = dp.GPIOA.split();
    let mut gpiob = dp.GPIOB.split();
    let mut afio = dp.AFIO.constrain();

    // USART1 on PA9 (TX) and PA10 (RX)
    let pa9 = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let pa10 = gpioa.pa10.into_floating_input(&mut gpioa.crh);
    let mut serial = Serial::usart1(
        dp.USART1,
        (pa9, pa10),
        &mut afio.mapr,
        Config::default().baudrate(115_200_u32.bps()),
        clocks,
    );

    // SIG_TRIGGER on PB0 — input, pulled down, rising edge from AND gate
    let mut trigger = gpiob.pb0.into_pull_down_input(&mut gpiob.crl);
    let mut led = gpiob.pb1.into_push_pull_output(&mut gpiob.crl);

    // Wait for the AND gate to assert the trigger
    loop {
        if trigger.is_high() {
            break;
        }
        cortex_m::asm::wfi();
    }

    // Blink LED to indicate trigger received
    for _ in 0..4 {
        led.toggle();
        cortex_m::asm::delay(1_000_000);
    }

    let key = b"BASTION_SIGNING_KEY_INTERNAL_XXXXXXXX";
    let mut rx_buf = [0u8; 256];
    let mut rx_pos = 0;

    loop {
        match serial.read() {
            Ok(byte) => {
                if byte == 0xAA {
                    rx_pos = 0;
                    rx_buf[rx_pos] = byte;
                    rx_pos += 1;
                } else if byte == 0x55 {
                    // End of frame: parse and sign the 104-byte message
                    if rx_pos >= 104 {
                        let msg = &rx_buf[1..105];
                        let sig = station::hmac_sha256(key, msg);
                        // Send back: 0xAA 0x01 32-byte sig 0x55
                        serial.write(0xAA).ok();
                        serial.write(0x01).ok();
                        for &b in sig.iter() {
                            serial.write(b).ok();
                        }
                        serial.write(0x55).ok();
                        serial.flush().ok();
                        // Blink LED
                        for _ in 0..4 {
                            led.toggle();
                            cortex_m::asm::delay(1_000_000);
                        }
                    }
                    rx_pos = 0;
                } else {
                    if rx_pos < rx_buf.len() {
                        rx_buf[rx_pos] = byte;
                        rx_pos += 1;
                    }
                }
            }
            Err(_) => cortex_m::asm::wfi(),
        }
    }
}

mod station {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
        mac.update(msg);
        let bytes = mac.finalize().into_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }
}
