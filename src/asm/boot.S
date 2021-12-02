# Disable generation of compressed instructions.
.option norvc

# Define a .text.init section. The .text.init is put at the
# starting address so that the entry _start is put at the RISC-V
# address 0x8000_0000.
.section .text.init

# Execution starts here.
.global _start
_start:

	# Disable linker instruction relaxation for the `la` instruction below.
	# This disallows the assembler from assuming that `gp` is already initialized.
	# This causes the value stored in `gp` to be calculated from `pc`.
	# The job of the global pointer is to give the linker the ability to address
	# memory relative to GP instead of as an absolute address.
.option push
.option norelax
	la		gp, _global_pointer
.option pop
	# SATP should be zero, but let's make sure. Each HART has its own
	# SATP register.
	csrw	satp, zero
	# Any hardware threads (hart) that are not bootstrapping
	# need to wait for an IPI
	csrr	t0, mhartid
	bnez	t0, 3f

	# The stack grows from bottom to top, so we put the stack pointer
	# to the very end of the stack range.
	la		sp, _stack_end

	# Setting `mstatus` register:
	# 0b01 << 11: Machine's previous protection mode is 2 (MPP=2). # out of date
	#li		t0, 0b11 << 11
	#csrw	mstatus, t0

	# set the previous mode to s mode for mret
	csrr	t0, mstatus
	li		t1, 3 << 11
	not		t1, t1
	and		t0, t0, t1
	li		t1, 1 << 11
	or		t0, t0, t1
	csrw	mstatus, t0

	# Do not allow interrupts while running kinit
	#csrw	mie, zero

	# Machine's exception program counter (MEPC) is set to `kinit`.
	la		t1, kinit
	csrw	mepc, t1

	# delegate all interrupts and exceptions to s mode
	li		t0, 0xffff
	csrw	medeleg, t0
	csrw	mideleg, t0
	csrr	t0, sie
	ori		t0, t0, 1 << 9
	ori		t0, t0, 1 << 5
	ori		t0, t0, 1 << 1
	csrw	sie, t0

	csrr	t0, mhartid
	mv		tp, t0

	call timer_init

	# Set the return address to get us into supervisor mode
	#la		ra, 2f
	# We use mret here so that the mstatus register is properly updated.
	mret
	#call kinit

3:
	j 3b