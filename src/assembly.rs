global_asm!(include_str!("asm/boot.S"));
global_asm!(include_str!("asm/trap.S"));
global_asm!(include_str!("asm/mem.S"));
global_asm!(include_str!("asm/kernelvec.S"));
global_asm!(include_str!("asm/switch.S"));
global_asm!(include_str!("asm/trampoline.S"));
