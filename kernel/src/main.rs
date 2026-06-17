#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec::Vec};
use bootloader_api::{BootInfo, entry_point, BootloaderConfig};
use core::panic::PanicInfo;
use bootloader_api::config::Mapping;
use bootloader_api::info::Optional;
use kernel::task::Task;
use kernel::task::executor::Executor;
use kernel::task::keyboard;
use kernel::{println, serial_println};
use x86_64::VirtAddr;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    use kernel::allocator;
    use kernel::memory::{self, BootInfoFrameAllocator};

    kernel::init();

    if let Some(framebuffer) =
        boot_info.framebuffer.as_mut()
    {
        kernel::framebuffer::init_framebuffer(
            framebuffer
        );
    }

    serial_println!("Hello World{}", "!");

    let physical_memory_offset = match boot_info.physical_memory_offset {
        Optional::Some(address) => address,
        Optional::None => panic!("map-physical-memory config option is disabled!")
    };

    let rsdp_addr = match boot_info.rsdp_addr {
        Optional::Some(address) => address as usize,
        Optional::None => panic!("rsdp_addr not find!")
    };

    unsafe {
        kernel::acpi::init(physical_memory_offset, rsdp_addr);
    }

    let mut mapper = unsafe { memory::init(VirtAddr::new(physical_memory_offset)) };

    let memory_regions = &boot_info.memory_regions;
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(memory_regions) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

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

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

async fn async_number() -> u32 {
    67
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use kernel::{eprintln, hlt_loop};
    
    eprintln!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}
