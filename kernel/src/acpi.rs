use acpi::{
    AcpiHandler, AcpiTables, PciConfigRegions, PhysicalMapping, PlatformInfo, hpet::HpetTable,
};
use core::ptr::NonNull;
use spin::Mutex;

extern crate alloc;

static ACPI_TABLES: Mutex<Option<AcpiTables<BootloaderAcpiHandler>>> = Mutex::new(None);
static PHYS_OFFSET: Mutex<Option<u64>> = Mutex::new(None);

#[derive(Clone, Copy)]
pub struct BootloaderAcpiHandler;

impl AcpiHandler for BootloaderAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let offset = PHYS_OFFSET.lock().unwrap() as usize;
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
    *PHYS_OFFSET.lock() = Some(physical_memory_offset);

    let tables = unsafe {
        AcpiTables::from_rsdp(BootloaderAcpiHandler, rsdp_addr)
            .expect("Failed to parse ACPI tables")
    };

    *ACPI_TABLES.lock() = Some(tables);
}

fn tables() -> spin::MutexGuard<'static, Option<AcpiTables<BootloaderAcpiHandler>>> {
    ACPI_TABLES.lock()
}

pub fn platform_info() -> PlatformInfo<'static, alloc::alloc::Global> {
    let guard = tables();
    let table = guard.as_ref().expect("ACPI not initialized");

    unsafe { core::mem::transmute(table.platform_info().expect("Failed to get platform info")) }
}

pub fn hpet() -> Option<PhysicalMapping<BootloaderAcpiHandler, HpetTable>> {
    let guard = tables();
    let table = guard.as_ref()?;
    table.find_table::<HpetTable>().ok()
}

pub fn pci_regions() -> Option<PciConfigRegions<'static, alloc::alloc::Global>> {
    let guard = tables();
    let table = guard.as_ref()?;
    unsafe { core::mem::transmute(PciConfigRegions::new_in(table, alloc::alloc::Global).ok()?) }
}

pub fn ssdts() -> alloc::vec::Vec<acpi::AmlTable> {
    let guard = tables();
    let table = guard.as_ref().expect("ACPI not initialized");
    table.ssdts().collect()
}

pub fn find_table<T: acpi::AcpiTable>() -> Option<PhysicalMapping<BootloaderAcpiHandler, T>> {
    let guard = tables();
    let table = guard.as_ref()?;
    table.find_table::<T>().ok()
}
