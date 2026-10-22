#![no_std]
#![no_main]

use cortex_m::peripheral::DWT;
use cortex_m_rt::entry;
use core::panic::PanicInfo;

use stm32f4xx_hal as hal;
use hal::pac;
use hal::prelude::*;

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const MSG_LEN: usize = 104; // 102-byte payload + 2 attestation flags

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

#[no_mangle]
pub unsafe extern "C" fn __pre_init() {}

// ── Exception handlers required by cortex-m-rt ─────────────────────────
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DefaultHandler(_irqn: i16) -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn NonMaskableInt() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HardFault() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn MemoryManagement() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn BusFault() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn UsageFault() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SVCall() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DebugMonitor() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PendSV() -> ! { loop { cortex_m::asm::bkpt(); } }
#[doc(hidden)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SysTick() -> ! { loop { cortex_m::asm::bkpt(); } }

// ── HMAC ───────────────────────────────────────────────────────────────
fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key");
    mac.update(msg);
    let bytes = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    out
}

// ── Entry ──────────────────────────────────────────────────────────────
#[entry]
fn main() -> ! {
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    let mut dwt = cp.DWT;
    dwt.enable_cycle_counter();

    let dp = pac::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.freeze(hal::rcc::Config::hsi());

    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut serial = dp
        .USART1
        .serial((gpioa.pa9, gpioa.pa10), 115200.bps(), &mut rcc)
        .unwrap();

    let key = b"BASTION_SIGNING_KEY_INTERNAL_XXXXXXXX";
    let mut rx_buf = [0u8; 256];
    let mut rx_pos: usize = 0;

    loop {
        match nb::block!(serial.read()) {
            Ok(byte) => {
                if byte == 0xAA {
                    rx_pos = 0;
                    rx_buf[rx_pos] = byte;
                    rx_pos += 1;
                } else if byte == 0x55 {
                    if rx_pos >= MSG_LEN + 1 {
                        let msg = &rx_buf[1..MSG_LEN + 1];
                        let start = DWT::cycle_count();
                        let sig = hmac_sha256(key, msg);
                        let end = DWT::cycle_count();
                        let cycles = end - start;
                        // Response: 0xAA 0x01 [32-byte sig] [4-byte cycles LE] 0x55
                        let _ = nb::block!(serial.write(0xAA));
                        let _ = nb::block!(serial.write(0x01));
                        for &b in sig.iter() {
                            let _ = nb::block!(serial.write(b));
                        }
                        for b in cycles.to_le_bytes().iter() {
                            let _ = nb::block!(serial.write(*b));
                        }
                        let _ = nb::block!(serial.write(0x55));
                    }
                    rx_pos = 0;
                } else {
                    if rx_pos < rx_buf.len() {
                        rx_buf[rx_pos] = byte;
                        rx_pos += 1;
                    }
                }
            }
            Err(_) => {}
        }
    }
}
