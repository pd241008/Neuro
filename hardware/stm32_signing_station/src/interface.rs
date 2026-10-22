// interface.rs — Interface traits for the STM32 signing station.
//
use nb::Error;

pub trait SigningStation {
    fn wait_for_trigger(&mut self) -> nb::Result<(), Error<()>>;
    fn send_message(&mut self, msg: &[u8]) -> nb::Result<(), Error<()>>;
    fn read_signature(&mut self) -> nb::Result<[u8; 32], Error<()>>;
}

pub trait AndGate {
    fn is_asserted(&self) -> bool;
}

pub trait UartSigner {
    fn send_frame(&mut self, frame: &[u8]) -> nb::Result<(), Error<()>>;
    fn recv_frame(&mut self) -> nb::Result<[u8; 256], Error<()>>;
}
