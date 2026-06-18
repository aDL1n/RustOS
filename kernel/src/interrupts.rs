use crate::{apic, eprintln, gdt, hlt_loop};
use lazy_static::lazy_static;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

            idt[apic::TIMER_VECTOR].set_handler_fn(timer_interrupt_handler);
            idt[apic::KEYBOARD_VECTOR].set_handler_fn(keyboard_interrupt_handler);
            idt[apic::ERROR_VECTOR].set_handler_fn(error_interrupt_handler);
            idt[apic::SPURIOUS_VECTOR].set_handler_fn(spurious_interrupt_handler);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt
    };
}

pub fn init() {
    IDT.load();
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    eprintln!("Exception: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "Exception: DOUBLE FAULT\n{:#?}\nerror code: {}\n",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        if let Some(ref mut local_apic_wrapper) = *apic::LOCAL_APIC.lock() {
            local_apic_wrapper.0.end_of_interrupt();
        }
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        if let Some(ref mut lapic) = *apic::LOCAL_APIC.lock() {
            lapic.0.end_of_interrupt();
        }
    }
}

extern "x86-interrupt" fn error_interrupt_handler(_stack_frame: InterruptStackFrame) {
    eprintln!("APIC Error Interrupt");
    unsafe {
        if let Some(ref mut local_apic_wrapper) = *apic::LOCAL_APIC.lock() {
            let flags = local_apic_wrapper.0.error_flags();
            eprintln!("APIC Error Flags: {:?}", flags);
            local_apic_wrapper.0.end_of_interrupt();
        }
    }
}

extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: InterruptStackFrame) {
    eprintln!("Spurious Interrupt");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    eprintln!("Exception: Page Fault!");
    eprintln!("Accessed Address: {:?}", Cr2::read());
    eprintln!("Error Code: {:?}", error_code);
    eprintln!("{:#?}", stack_frame);
    hlt_loop();
}