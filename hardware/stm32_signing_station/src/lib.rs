#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

pub mod geometric;
pub mod interface;
pub mod uart_bridge;

// Re-export the high-level API
pub use geometric::AndGate;
pub use interface::{SigningStation, UartSigner};
pub use uart_bridge::UartBridge;

// The actual firmware entry point is in src/bin/station.rs
// or src/main.rs depending on how the project is set up.
// For now, this crate is a library that the firmware binary uses.

// The firmware binary is not included in this crate because it
// requires the stm32f1xx-hal crate, which is not a dependency
// of this crate.  The firmware binary is a separate crate that
// depends on this crate and the hal.

// The host-side benchmark is in src/bin/host_bench.rs.
// It uses the same HMAC-SHA256 implementation as the firmware,
// but runs on the host for verification of the timing numbers.
