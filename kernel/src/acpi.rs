extern crate alloc;
use acpi::madt::{Madt, MadtEntryIter};
use acpi::{
    AcpiHandler, AcpiTables, PciConfigRegions, PhysicalMapping, PlatformInfo, hpet::HpetTable,
};
use core::ptr::NonNull;
use spin::Once;

pub struct SafeAcpiTables(pub AcpiTables<BootloaderAcpiHandler>);

unsafe impl Sync for SafeAcpiTables {}

static ACPI_TABLES: Once<SafeAcpiTables> = Once::new();
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
        let tables = AcpiTables::from_rsdp(BootloaderAcpiHandler, rsdp_addr)
            .expect("Failed to parse ACPI tables");
        SafeAcpiTables(tables)
    });
}

fn tables() -> &'static AcpiTables<BootloaderAcpiHandler> {
    &ACPI_TABLES.get().expect("ACPI_TABLES not initialized").0
}

pub fn platform_info() -> PlatformInfo<'static, alloc::alloc::Global> {
    tables()
        .platform_info()
        .expect("Failed to get platform info")
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
