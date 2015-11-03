/*
            Copyright Oliver Kowalke 2009.
   Distributed under the Boost Software License, Version 1.0.
      (See accompanying file LICENSE_1_0.txt or copy at
            http://www.boost.org/LICENSE_1_0.txt)
*/

/****************************************************************************************
 *                                                                                      *
 *  ----------------------------------------------------------------------------------  *
 *  |    0    |    1    |    2    |    3    |    4     |    5    |    6    |    7    |  *
 *  ----------------------------------------------------------------------------------  *
 *  |   0x0   |   0x4   |   0x8   |   0xc   |   0x10   |   0x14  |   0x18  |   0x1c  |  *
 *  ----------------------------------------------------------------------------------  *
 *  | fc_mxcsr|fc_x87_cw|        R12        |         R13        |        R14        |  *
 *  ----------------------------------------------------------------------------------  *
 *  ----------------------------------------------------------------------------------  *
 *  |    8    |    9    |   10    |   11    |    12    |    13   |    14   |    15   |  *
 *  ----------------------------------------------------------------------------------  *
 *  |   0x20  |   0x24  |   0x28  |  0x2c   |   0x30   |   0x34  |   0x38  |   0x3c  |  *
 *  ----------------------------------------------------------------------------------  *
 *  |        R15        |        RBX        |         RBP        |        RIP        |  *
 *  ----------------------------------------------------------------------------------  *
 *  normal stack                                                                        *
 *  ----------------------------------------------------------------------------------  *
 *  |    16   |   17    |                                                            |  *
 *  ----------------------------------------------------------------------------------  *
 *  |   0x40  |   0x44  |                                                            |  *
 *  ----------------------------------------------------------------------------------  *
 *  |        EXIT       |                                                            |  *
 *  ----------------------------------------------------------------------------------  *
 *  stack when entering coroutine func for the first time                               *
 *  ----------------------------------------------------------------------------------  *
 *  |    16   |   17    |   18    |   19    |    20    |   21    |                   |  *
 *  ----------------------------------------------------------------------------------  *
 *  |   0x40  |   0x44  |  0x48   |  0x4c   |   0x50   |  0x54   |                   |  *
 *  ----------------------------------------------------------------------------------  *
 *  | CONTEXT_REF(arg1) |  FUNC_ADDR (JMP)  |        EXIT        |                   |  *
 *  ----------------------------------------------------------------------------------  *
 ****************************************************************************************/

.text
.globl jump_fcontext
.type jump_fcontext,@function
.align 16
jump_fcontext:
    /*
        allow null pointer in first arg for one-way jumps
    */
    cmp    $0, %rdi
    je     1f

    /*
        Store section
    */
    pushq  %rbp  /* save RBP */
    pushq  %rbx  /* save RBX */
    pushq  %r15  /* save R15 */
    pushq  %r14  /* save R14 */
    pushq  %r13  /* save R13 */
    pushq  %r12  /* save R12 */

    /* prepare stack for FPU */
    leaq  -0x8(%rsp), %rsp

    /* save MMX control- and status-word */
    stmxcsr  (%rsp)
    /* save x87 control-word */
    fnstcw   0x4(%rsp)

    /* store RSP (pointing to context-data) in RDI */
    movq  %rsp, (%rdi)

1:
    /*
        Load section
    */
    /* restore RSP (pointing to context-data) from RSI */
    movq  %rsi, %rsp

    /* restore MMX control- and status-word */
    ldmxcsr  (%rsp)
    /* restore x87 control-word */
    fldcw  0x4(%rsp)

    /* clear stack of FPU */
    leaq  0x8(%rsp), %rsp

    popq  %r12  /* restrore R12 */
    popq  %r13  /* restrore R13 */
    popq  %r14  /* restrore R14 */
    popq  %r15  /* restrore R15 */
    popq  %rbx  /* restrore RBX */
    popq  %rbp  /* restrore RBP */

    /* use third arg as return-value after jump */
    movq  %rdx, %rax

    /* at this point, return address is on top of stack, so we can simply ret */
    ret
.size jump_fcontext,.-jump_fcontext

/* Mark that we don`t need executable stack.  */
.section .note.GNU-stack,"",%progbits

/*
  There are some changes to how make_fcontext works
  It`d be beneficial to pass some argument to initial
  function, which depends on initial construction func
  and not on coroutine we came from, i.e. some void* data
  - jump_fcontext doesn`t try to serve start point anymore
  - init func isn`t invoked immediately,
    micro-trampoline is used instead
  - trampoline copies 'return value' into 1st argument, %rdi,
    then pops value from stack into 2nd argument, %rsi,
    then pops next value into %r8 and jumps to it
*/
.text
.globl make_fcontext
.type make_fcontext,@function
.align 16
make_fcontext:
    /* first arg of make_fcontext() == top of context-stack */
    movq  %rdi, %rax

    /* shift address in RAX to lower 16 byte boundary */
    andq  $-16, %rax

    /* reserve space for context-data on context-stack */
    /* size for fc_mxcsr .. RIP + data + context-function pointer */
    /* + return-address for context-function */
    /* on context-function entry: (RSP -0x8) % 16 == 0 */
    leaq  -0x58(%rax), %rax

    /* third arg of make_fcontext() == address of context-function */
    movq  %rdx, 0x48(%rax)
    /* fourth arg of make_fcontext() == data arg for context-function */
    movq  %rcx, 0x40(%rax)
    /* compute abs address of label trampoline */
    leaq  trampoline(%rip), %r8
    /* and use it as stored RIP */
    movq  %r8, 0x38(%rax)

    /* save MMX control- and status-word */
    stmxcsr  (%rax)
    /* save x87 control-word */
    fnstcw   0x4(%rax)

    /* compute abs address of label finish */
    leaq  finish(%rip), %r8
    /* save address of finish as return-address for context-function */
    /* will be entered after context-function returns */
    movq  %r8, 0x50(%rax)

    ret /* return pointer to context-data */

finish:
    /* exit code is zero */
    xorq  %rdi, %rdi
    /* exit application */
    call  _exit@PLT
    hlt

trampoline:
    movq  %rax, %rdi /* reinterpret retval as arg0, message        */
    popq  %rsi       /* restore 'data' from stack into arg1        */
    ret              /* equivalent to popq+jmp, execute context fn */

.size make_fcontext,.-make_fcontext

/* Mark that we don`t need executable stack. */
.section .note.GNU-stack,"",%progbits
