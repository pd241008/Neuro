// uart_bridge.rs — UART bridge implementation for the STM32 signing station.
//
use embedded_hal::serial::Read;
use embedded_hal::serial::Write;
use nb::Error;

const START_MARKER: u8 = 0xAA;
const END_MARKER: u8 = 0x55;
const STATUS_OK: u8 = 0x01;
const STATUS_ERROR: u8 = 0x00;

pub struct UartBridge<UART> {
    uart: UART,
}

impl<UART: Read<u8, Error = Error<()>> + Write<u8, Error = Error<()>>> UartBridge<UART> {
    pub fn new(uart: UART) -> Self {
        Self { uart }
    }

    pub fn send_frame(&mut self, frame: &[u8]) -> nb::Result<(), Error<()>> {
        self.uart.write(START_MARKER)?;
        for &byte in frame {
            self.uart.write(byte)?;
        }
        self.uart.write(END_MARKER)?;
        self.uart.flush()?;
        Ok(())
    }

    pub fn recv_frame(&mut self) -> nb::Result<[u8; 256], Error<()>> {
        let mut frame = [0u8; 256];
        let mut pos = 0;
        loop {
            match self.uart.read() {
                Ok(byte) => {
                    if byte == START_MARKER {
                        pos = 0;
                    } else if byte == END_MARKER {
                        return Ok(frame);
                    } else if pos < frame.len() {
                        frame[pos] = byte;
                        pos += 1;
                    }
                }
                Err(nb::Error::WouldBlock) => return Err(nb::Error::WouldBlock),
                Err(nb::Error::Other(err)) => return Err(nb::Error::Other(err)),
            }
        }
    }

    pub fn send_status(&mut self, status: u8) -> nb::Result<(), Error<()>> {
        self.uart.write(START_MARKER)?;
        self.uart.write(status)?;
        self.uart.write(END_MARKER)?;
        self.uart.flush()?;
        Ok(())
    }

    pub fn recv_status(&mut self) -> nb::Result<u8, Error<()>> {
        let mut status: Option<u8> = None;
        loop {
            match self.uart.read() {
                Ok(byte) => {
                    if byte == START_MARKER {
                        status = None;
                    } else if byte == END_MARKER {
                        return match status {
                            Some(s) => Ok(s),
                            None => Err(nb::Error::WouldBlock),
                        };
                    } else if status.is_none() {
                        status = Some(byte);
                    }
                }
                Err(nb::Error::WouldBlock) => return Err(nb::Error::WouldBlock),
                Err(nb::Error::Other(err)) => return Err(nb::Error::Other(err)),
            }
        }
    }
}
