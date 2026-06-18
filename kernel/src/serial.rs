use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::backend::PioBackend;
use uart_16550::{Config, Uart16550Tty};

lazy_static! {
    pub static ref SERIAL1: Mutex<Uart16550Tty<PioBackend>> = {
        let serial_port = unsafe { 
            Uart16550Tty::new_port(0x3F8, Config::default()).unwrap() 
        };
        
        Mutex::new(serial_port)
    };
}
