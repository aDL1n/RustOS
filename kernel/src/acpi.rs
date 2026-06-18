extern crate alloc;
use acpi::madt::{Madt, MadtEntryIter};
use acpi::{hpet::HpetTable, AcpiHandler, AcpiTables, PciConfigRegions, PhysicalMapping, PlatformInfo};
use core::ptr::NonNull;
use spin::Once;

static ACPI_TABLES: Once<AcpiTables<BootloaderAcpiHandler>> = Once::new();
static PHYS_OFFSET: Once<u64> = Once::new();

#[derive(Clone, Copy)]
pub struct BootloaderAcpiHandler;

impl AcpiHandler for BootloaderAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let offset = *PHYS_OFFSET.get().expect("PHYS_OFFSET not initialized") as usize;
        let virtual_address = physical_address + offset;

        unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new(virtual_address as *mut T).unwrap(),
                size,
                size,
                *self,
            )
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}

pub unsafe fn init(physical_memory_offset: u64, rsdp_addr: usize) {
    PHYS_OFFSET.call_once(|| physical_memory_offset);

    ACPI_TABLES.call_once(|| unsafe {
        AcpiTables::from_rsdp(BootloaderAcpiHandler, rsdp_addr)
            .expect("Failed to parse ACPI tables")
    });
}

fn tables() -> &'static AcpiTables<BootloaderAcpiHandler> {
    ACPI_TABLES.get().expect("ACPI_TABLES not initialized")
}

pub fn platform_info() -> PlatformInfo<'static, alloc::alloc::Global> {
    tables().platform_info().expect("Failed to get platform info")
}

pub fn hpet() -> Option<PhysicalMapping<BootloaderAcpiHandler, HpetTable>> {
    find_table::<HpetTable>()
}

pub fn pci_regions() -> Option<PciConfigRegions<'static, alloc::alloc::Global>> {
    PciConfigRegions::new_in(tables(), alloc::alloc::Global).ok()
}

pub fn ssdts() -> alloc::vec::Vec<acpi::AmlTable> {
    tables().ssdts().collect()
}

pub fn madt() -> Option<PhysicalMapping<BootloaderAcpiHandler, Madt>> {
    find_table::<Madt>()
}

pub fn find_table<T: acpi::AcpiTable>() -> Option<PhysicalMapping<BootloaderAcpiHandler, T>> {
    tables().find_table::<T>().ok()
}