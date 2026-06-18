#![no_std]
#![no_main]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use bootloader_api::config::Mapping;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use core::panic::PanicInfo;
use kernel::task::executor::Executor;
use kernel::task::keyboard;
use kernel::task::Task;
use kernel::{hlt_loop, println};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    // config.kernel_stack_size = 200 * 1024;
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kernel::init(boot_info);

    println!("Hello World{}", "!");

    let heap_value = Box::new(42);
    println!("heap value: {:p}", heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at: {:p}", vec.as_slice());

    let reference_counted = Rc::new(vec);
    let cloned_reference = reference_counted.clone();
    println!("reference counted: {:p}", reference_counted);
    core::mem::drop(reference_counted);
    println!("cloned reference: {:p}", cloned_reference);

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));

    println!("all works good!");

    executor.run();
}

async fn async_number() -> u32 {
    67
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use kernel::{eprintln, hlt_loop};

    eprintln!("{}", info);
    hlt_loop()
}
