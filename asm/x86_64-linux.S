/*
According to ABI documentation found at
http://www.x86-64.org/documentation.html
and Microsoft discussion at
http://msdn.microsoft.com/en-US/library/9z1stfyw%28v=VS.80%29.aspx.

Callee save registers:
	R12--R15, RBX, RBP, RSP, XMM0--XMM5 (?)

Integer arguments go in registers:
        rdi, rsi, rdx, rcx, r8, r9

User flags have no specified role and are not preserved
        across calls, with the exception of DF in %rFLAGS,
        which must be clear (set to "forward" direction)
        on function entry and return.
*/

/*
    Storage structure:

    ---------------------------------------------------------
    | 0x00 | 0x04 | 0x08 | 0x0C | 0x10 | 0x14 | 0x18 | 0x1C |
    ---------------------------------------------------------
    |     RIP     |     RSP     |     RBP     |     RBX     |
    ---------------------------------------------------------
    ---------------------------------------------------------
    | 0x20 | 0x24 | 0x28 | 0x2C | 0x30 | 0x34 | 0x38 | 0x3C |
    ---------------------------------------------------------
    |     R12     |     R13     |     R14     |     R15     |
    ---------------------------------------------------------
    ---------------------------------------------------------
    | 0x40 | 0x44 | 0x48 | 0x4C | 0x50 | 0x54 | 0x58 | 0x5C |
    ---------------------------------------------------------
    | x87f | MMXf |                                         |
    ---------------------------------------------------------
*/

#define OFF_RIP 0x00
#define OFF_RSP 0x08
#define OFF_RBP 0x10
#define OFF_RBX 0x18

#define OFF_R12 0x20
#define OFF_R13 0x28
#define OFF_R14 0x30
#define OFF_R15 0x38

#define OFF_X87 0x40
#define OFF_MMX 0x44

/* Present here mostly for reference */
#define CONTEXT_SIZE 0x48

/*
    Her Majesty Code Section :)
*/
.text
/*
    void context_rs_swap(context_t *store, context_t *load)

    Stores current state into buffer pointed by store,
    and restores state from buffer pointed by load

    If any of pointers is null, then appropriate action is skipped
*/
#define STORE %rdi
#define LOAD  %rsi

.section .note.GNU-stack, "", %progbits
.global  context_rs_swap
.type    context_rs_swap,@function
.align   16
context_rs_swap:
    /*
        Sometimes we need to consume ret IP, and sometimes jump to it
        So we just pop it into RAX unconditionally
    */
    popq %rax
    /*
        If nowhere to save, skip
    */
    cmp  $0, STORE
    je   1f
    /*
        Store return IP from RAX
    */
    movq %rax, (OFF_RIP)(STORE)

    /*
        Next, store non-volatile registers
    */
    movq %rsp, (OFF_RSP)(STORE)
    movq %rbp, (OFF_RBP)(STORE)
    movq %rbx, (OFF_RBX)(STORE)

    movq %r12, (OFF_R12)(STORE)
    movq %r13, (OFF_R13)(STORE)
    movq %r14, (OFF_R14)(STORE)
    movq %r15, (OFF_R15)(STORE)

    fnstcw  (OFF_X87)(STORE)
    stmxcsr (OFF_MMX)(STORE)

1:
    /*
        If nowhere to load from, return
    */
    cmp  $0, LOAD
    je   2f
    jmp  *%rax

2:
    /*
        Restore NV registers, except RIP
    */
    movq (OFF_RSP)(LOAD), %rsp
    movq (OFF_RBP)(LOAD), %rbp
    movq (OFF_RBX)(LOAD), %rbx

    movq (OFF_R12)(LOAD), %r12
    movq (OFF_R13)(LOAD), %r13
    movq (OFF_R14)(LOAD), %r14
    movq (OFF_R15)(LOAD), %r15

    fldcw   (OFF_X87)(LOAD)
    ldmxcsr (OFF_MMX)(LOAD)
    /*
        Finally, get destination RIP and jump there
    */
    jmp *(OFF_RIP)(LOAD)

.size context_rs_swap,.-context_rs_swap

#undef STORE
#undef LOAD

/*
    void context_rs_init(
        context_t* storage,
        void* stack_base,
        size_t stack_size,
        void (*func)(intptr_t),
        intptr_t data
    )

    Initializes empty instance of context_t,
    with heapstack parameters and execution routine
    Assumes that context_t is big enough and is zeroed, in general case

    The main idea is:
    1. Place halt address as return for context func onto stack
    2. Store stack_base at OFF_RSP
    3. Possibly, set stack bounds
    4. set OFF_RIP to special thunk section
*/
#define CONTEXT %rdi
#define ST_BASE %rsi
#define ST_SIZE %rdx
#define CLOSURE %rcx
#define CL_ARG  %r8

.section .note.GNU-stack, "", %progbits
.global  context_rs_init
.type    context_rs_init,@function
.align   16
context_rs_init:
    /* align SP to 16-byte boundary */
    andq $-16, ST_BASE
    /* reserve space for halt ret address and put SP */
    leaq -0x8(ST_BASE), ST_BASE
    movq ST_BASE, (OFF_RSP)(CONTEXT)
    /* place halt; use CL_ARG5 %r9 as buffer since it's not used */
    leaq halt(%rip), %r9
    movq %r9, 0x8(ST_BASE)
    /* place thunk address as start point */
    leaq thunk(%rip), %r9
    movq %r9, (OFF_RIP)(CONTEXT)
    /* store fn ptr in r12  */
    movq CLOSURE, (OFF_R12)(CONTEXT)
    /* store data in r13  */
    movq CL_ARG,  (OFF_R13)(CONTEXT)
    /* x87 and MMX state */
    fnstcw  (OFF_X87)(CONTEXT)
    stmxcsr (OFF_MMX)(CONTEXT)

    ret
/*
    Thunk reads closure argument from %r13 into %rdi
    Then, jumps to closure start stored at %r12
*/
thunk:
    movq %r12, %rdi
    jmp  *%r13
/*
    halt is a special jump point, which is placed onto context's pseudo-stack
    as return address, so context func will return to it when finishes
    halt terminates running process
    
    the more effective way to enforce no-return is to use fn (intptr_t) -> !
    as context function type
*/
halt:   
    /* exit code is zero */
    xorq  %rdi, %rdi
    /* exit application */
    call  _exit@PLT
    hlt
.size context_rs_init,.-context_rs_init
