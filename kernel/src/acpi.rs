extern crate alloc;

use core::ptr::{read_volatile, write_volatile, NonNull};
use acpi::{AcpiTables, Handle, Handler, PciAddress, PhysicalMapping};
use acpi::aml::AmlError;
use acpi::platform::AcpiPlatform;
use acpi::sdt::madt::Madt;
use spin::Once;

static ACPI_PLATFORM: Once<AcpiPlatform<BootloaderAcpiHandler>> = Once::new();
static PHYS_OFFSET: Once<u64> = Once::new();

#[derive(Clone, Copy)]
pub struct BootloaderAcpiHandler;

impl Handler for BootloaderAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let offset = *PHYS_OFFSET.get().expect("PHYS_OFFSET not initialized") as usize;
        let virtual_address = physical_address + offset;

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(virtual_address as *mut T).unwrap(),
            region_length: size,
            mapped_length: size,
            handler: *self,
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}

    fn read_u8(&self, address: usize) -> u8 {
        unsafe { read_volatile(address as *const u8) }
    }

    fn read_u16(&self, address: usize) -> u16 {
        unsafe { read_volatile(address as *const u16) }
    }

    fn read_u32(&self, address: usize) -> u32 {
        unsafe { read_volatile(address as *const u32) }
    }

    fn read_u64(&self, address: usize) -> u64 {
        unsafe { read_volatile(address as *const u64) }
    }

    fn write_u8(&self, address: usize, value: u8) {
        unsafe { write_volatile(address as *mut u8, value) }
    }

    fn write_u16(&self, address: usize, value: u16) {
        unsafe { write_volatile(address as *mut u16, value) }
    }

    fn write_u32(&self, address: usize, value: u32) {
        unsafe { write_volatile(address as *mut u32, value) }
    }

    fn write_u64(&self, address: usize, value: u64) {
        unsafe { write_volatile(address as *mut u64, value) }
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        let value: u8;
        unsafe {
            core::arch::asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack));
        }
        value
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        let value: u16;
        unsafe {
            core::arch::asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack));
        }
        value
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        let value: u32;
        unsafe {
            core::arch::asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack));
        }
        value
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
        }
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        unsafe {
            core::arch::asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack));
        }
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        unsafe {
            core::arch::asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack));
        }
    }

    fn read_pci_u8(&self, address: PciAddress, offset: u16) -> u8 {
        let addr = pci_config_address(address, offset);
        self.write_io_u32(0xCF8, addr);
        ((self.read_io_u32(0xCFC) >> ((offset & 3) * 8)) & 0xFF) as u8
    }

    fn read_pci_u16(&self, address: PciAddress, offset: u16) -> u16 {
        let addr = pci_config_address(address, offset);
        self.write_io_u32(0xCF8, addr);
        ((self.read_io_u32(0xCFC) >> ((offset & 2) * 8)) & 0xFFFF) as u16
    }

    fn read_pci_u32(&self, address: PciAddress, offset: u16) -> u32 {
        let addr = pci_config_address(address, offset);
        self.write_io_u32(0xCF8, addr);
        self.read_io_u32(0xCFC)
    }

    fn write_pci_u8(&self, address: PciAddress, offset: u16, value: u8) {
        let addr = pci_config_address(address, offset);
        self.write_io_u32(0xCF8, addr);
        let shift = (offset & 3) * 8;
        let mask = !(0xFFu32 << shift);
        let current = self.read_io_u32(0xCFC) & mask;
        self.write_io_u32(0xCFC, current | ((value as u32) << shift));
    }

    fn write_pci_u16(&self, address: PciAddress, offset: u16, value: u16) {
        let addr = pci_config_address(address, offset);
        self.write_io_u32(0xCF8, addr);
        let shift = (offset & 2) * 8;
        let mask = !(0xFFFFu32 << shift);
        let current = self.read_io_u32(0xCFC) & mask;
        self.write_io_u32(0xCFC, current | ((value as u32) << shift));
    }

    fn write_pci_u32(&self, address: PciAddress, offset: u16, value: u32) {
        let addr = pci_config_address(address, offset);
        self.write_io_u32(0xCF8, addr);
        self.write_io_u32(0xCFC, value);
    }

    fn nanos_since_boot(&self) -> u64 {
        0
    }

    fn stall(&self, microseconds: u64) {
        for _ in 0..microseconds * 100 {
            unsafe { core::arch::asm!("pause", options(nomem, nostack)); }
        }
    }

    fn sleep(&self, milliseconds: u64) {
        self.stall(milliseconds * 1000);
    }

    fn create_mutex(&self) -> Handle {
        Handle(0)
    }

    fn acquire(&self, mutex: Handle, timeout: u16) -> Result<(), AmlError> {
        Ok(())
    }

    fn release(&self, mutex: Handle) {

    }
}

pub unsafe fn init(physical_memory_offset: u64, rsdp_addr: usize) {
    PHYS_OFFSET.call_once(|| physical_memory_offset);

    ACPI_PLATFORM.call_once(|| unsafe {
        let tables = AcpiTables::from_rsdp(BootloaderAcpiHandler, rsdp_addr)
            .expect("Failed to parse ACPI tables");

        let apic_platform = AcpiPlatform::new(tables, BootloaderAcpiHandler)
            .expect("Failed to parse ACPI platform");
        
        apic_platform
    });
}

pub fn tables() -> &'static AcpiTables<BootloaderAcpiHandler> {
    &ACPI_PLATFORM.get().expect("ACPI_TABLES not initialized").tables
}

pub fn pci_config_regions() -> acpi::platform::pci::PciConfigRegions {
    acpi::platform::pci::PciConfigRegions::new(tables()).unwrap()
}

fn pci_config_address(address: PciAddress, offset: u16) -> u32 {
    assert_eq!(address.segment(), 0, "PCI segments > 0 require MCFG/MMIO access");

    ((address.bus() as u32) << 16)
        | ((address.device() as u32) << 11)
        | ((address.function() as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x8000_0000
}

pub fn platform() -> &'static AcpiPlatform<BootloaderAcpiHandler> {
    ACPI_PLATFORM.get().expect("platform_info not initialized")
}

pub fn madt() -> PhysicalMapping<BootloaderAcpiHandler, Madt>{
    tables().find_table::<Madt>().unwrap()
}