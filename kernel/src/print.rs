use core::fmt;
use crate::framebuffer::WRITER;
use crate::serial::SERIAL1;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => ($crate::print::_eprint(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprintln {
    () => {$crate::eprint!("\n")};
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}


#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::print::_serial_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! serial_eprint {
    ($($arg:tt)*) => {
        $crate::print::_serial_eprint(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_eprintln {
    () => ($crate::serial_eprint!("\n"));
    ($fmt:expr) => ($crate::serial_eprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_eprint!(
        concat!($fmt, "\n"), $($arg)*));
}

#[doc(hidden)]
pub fn _serial_print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

#[doc(hidden)]
pub fn _serial_eprint(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        let mut serial = SERIAL1.lock();
        let _ = serial.write_str("\x1b[31m");
        serial.write_fmt(args).expect("Printing to serial failed");
        let _ = serial.write_str("\x1b[0m");
    });
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use x86_64::instructions::interrupts;
    
    _serial_print(args);

    interrupts::without_interrupts(|| {
        if let Some(writer) = WRITER.lock().as_mut() {
            use core::fmt::Write;
            let _ = writer.write_fmt(args);
        }
    });
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    _serial_eprint(args);

    interrupts::without_interrupts(|| {
        if let Some(writer) = WRITER.lock().as_mut() {
            let old_color = writer.color;
            writer.set_color(255, 0, 0);

            let _ = writer.write_fmt(args);

            writer.color = old_color;
        }
    });
}

