// Mark stack as non-executable
#if defined(__linux__) && defined(__ELF__)
.section	.note.GNU-stack, "", %progbits
#endif

.text
.code 32
.arm
#if defined(__APPLE__)
.align 2
#else
.align
#endif

#if defined(__APPLE__)
  #define SWAP_REGISTERS  _rust_swap_registers
  #define BOOTSTRAP_TASK  _rust_bootstrap_green_task
  #define LOAD_REGISTERS  _rust_load_registers
  #define RECORD_SP_LIMIT _record_sp_limit
  #define GET_SP_LIMIT    _get_sp_limit
#else
  #define SWAP_REGISTERS  rust_swap_registers
  #define BOOTSTRAP_TASK  rust_bootstrap_green_task
  #define LOAD_REGISTERS  rust_load_registers
  #define RECORD_SP_LIMIT record_sp_limit
  #define GET_SP_LIMIT    get_sp_limit
#endif

.globl SWAP_REGISTERS
SWAP_REGISTERS:
	str r0, [r0, #0]
	str r3, [r0, #12]
	str r4, [r0, #16]
	str r5, [r0, #20]
	str r6, [r0, #24]
	str r7, [r0, #28]
	str r8, [r0, #32]
	str r9, [r0, #36]
	str r10, [r0, #40]
	str r11, [r0, #44]
	str r12, [r0, #48]
	str sp, [r0, #52]
	str lr, [r0, #56]

	mrs r2, cpsr
	str r2, [r0, #64]


	ldr r0, [r1, #0]
	ldr r3, [r1, #12]
	ldr r4, [r1, #16]
	ldr r5, [r1, #20]
	ldr r6, [r1, #24]
	ldr r7, [r1, #28]
	ldr r8, [r1, #32]
	ldr r9, [r1, #36]
	ldr r10, [r1, #40]
	ldr r11, [r1, #44]
	ldr r12, [r1, #48]

	ldr sp, [r1, #52]
	ldr lr, [r1, #56]

	ldr r2, [r1, #64]
	msr cpsr_cxsf, r2

	mov pc, lr

// load_registers(registers_t *regs)
.globl LOAD_REGISTERS
LOAD_REGISTERS:
    ldr r0, [r0, #0]
	ldr r3, [r0, #12]
	ldr r4, [r0, #16]
	ldr r5, [r0, #20]
	ldr r6, [r0, #24]
	ldr r7, [r0, #28]
	ldr r8, [r0, #32]
	ldr r9, [r0, #36]
	ldr r10, [r0, #40]
	ldr r11, [r0, #44]
	ldr r12, [r0, #48]

	ldr sp, [r0, #52]
	ldr lr, [r0, #56]

	ldr r2, [r0, #64]
	msr cpsr_cxsf, r2

	mov pc, lr

// For reasons of this existence, see the comments in x86_64/_context.S
.globl BOOTSTRAP_TASK
BOOTSTRAP_TASK:
	mov r0, r0
	mov r1, r3
// no need for the second argument
//    mov r2, r4
    mov pc, r5

.globl 	RECORD_SP_LIMIT
RECORD_SP_LIMIT:
	// First, try to read TLS address from coprocessor
	mrc p15, #0, r3, c13, c0, #3
	cmp r3, #0
	// Otherwise, try to read from magic address 0xFFFF0FF0
	mvneq r3, #0xF000
	ldreq r3, [r3, #-15]

#if __ANDROID__
	add r3, r3, #252
#elif __linux__
	add r3, r3, #4
#endif // ANDROID

	str r0, [r3]
	mov pc, lr

.globl	GET_SP_LIMIT
GET_SP_LIMIT:
	// First, try to read TLS address from coprocessor
	mrc p15, #0, r3, c13, c0, #3
	cmp r3, #0
	// Otherwise, try to read from magic address 0xFFFF0FF0
	mvneq r3, #0xF000
	ldreq r3, [r3, #-15]

#if __ANDROID__
	add r3, r3, #252
#elif __linux__
	add r3, r3, #4
#endif  // __ANDROID__

	ldr r0, [r3]
	mov pc, lr
#endif
