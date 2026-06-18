use acpi::platform::interrupt::Apic;
use acpi::platform::InterruptModel;
use spin::Mutex;
use x2apic::ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry};
use x2apic::lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode};

pub const TIMER_VECTOR: u8 = 32;
pub const ERROR_VECTOR: u8 = 33;
pub const SPURIOUS_VECTOR: u8 = 34;
pub const KEYBOARD_VECTOR: u8 = 35;

pub struct LocalApicWrapper(pub LocalApic);
unsafe impl Send for LocalApicWrapper {}

pub static LOCAL_APIC: Mutex<Option<LocalApicWrapper>> = Mutex::new(None);
pub static IO_APIC: Mutex<Option<IoApic>> = Mutex::new(None);

pub fn init(phys_offset: u64) {
    let apic_info: Option<&Apic> = match crate::acpi::platform().interrupt_model {
        InterruptModel::Apic(ref apic) => Some(apic),
        _ => None,
    };

    init_io_apic(phys_offset, apic_info);
    init_local_apic(phys_offset, apic_info);
}

pub fn end_of_interrupt() {
    unsafe {
        if let Some(ref mut lapic) = *LOCAL_APIC.lock() {
            lapic.0.end_of_interrupt();
        }
    }
}

fn init_io_apic(phys_offset: u64, apic_info: Option<&Apic>) {
    let ioapic_phys_addr = apic_info
        .unwrap()
        .io_apics
        .first()
        .map(|ioapic| ioapic.address as u64)
        .unwrap_or(0xFEC0_0000);

    unsafe {
        let mut ioapic = IoApic::new(ioapic_phys_addr + phys_offset);
        ioapic.init(TIMER_VECTOR);

        let mut entry = RedirectionTableEntry::default();
        entry.set_vector(KEYBOARD_VECTOR);
        entry.set_mode(IrqMode::Fixed);
        entry.set_flags(IrqFlags::empty());
        entry.set_dest(0);

        ioapic.set_table_entry(1, entry);
        ioapic.enable_irq(1);

        *IO_APIC.lock() = Some(ioapic);
    }
}

fn init_local_apic(phys_offset: u64, apic_info: Option<&Apic>) {
    let local_apic_phys_adds = apic_info.unwrap().local_apic_address;

    let lapic = LocalApicBuilder::new()
        .set_xapic_base(local_apic_phys_adds + phys_offset)
        .timer_vector(TIMER_VECTOR as usize)
        .error_vector(ERROR_VECTOR as usize)
        .spurious_vector(SPURIOUS_VECTOR as usize)
        .timer_mode(TimerMode::Periodic)
        .timer_divide(TimerDivide::Div16)
        .timer_initial(10_000_000)
        .build()
        .expect("failed to build LocalApic");

    unsafe {
        let mut wrapper = LocalApicWrapper(lapic);
        wrapper.0.enable();
        wrapper.0.enable_timer();
        *LOCAL_APIC.lock() = Some(wrapper);
    }
}