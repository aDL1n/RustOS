#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
extern crate alloc;

use crate::memory::BootInfoFrameAllocator;
use bootloader_api::info::Optional;
use bootloader_api::BootInfo;
use x86_64::VirtAddr;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;
pub mod task;
pub mod framebuffer;
pub mod acpi;

pub fn init(boot_info: &'static mut BootInfo) {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    let physical_memory_offset = match boot_info.physical_memory_offset {
        Optional::Some(address) => address,
        Optional::None => panic!("map-physical-memory config option is disabled!"),
    };

    let rsdp_addr = match boot_info.rsdp_addr {
        Optional::Some(address) => address as usize,
        Optional::None => panic!("rsdp_addr not find!"),
    };
    
    let mut mapper = unsafe { memory::init(VirtAddr::new(physical_memory_offset)) };

    let memory_regions = &boot_info.memory_regions;
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(memory_regions) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        framebuffer::init_framebuffer(framebuffer);
    }
    
    unsafe {
        acpi::init(physical_memory_offset, rsdp_addr);
    }
    
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
