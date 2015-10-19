use libc;

type Ptr = *mut libc::c_void;

struct Buffer([u8; context_rs_context_size]);

extern {

    fn context_rs_swap(from: Ptr, to: Ptr);
    fn context_rs_init(
        ctxt:       Ptr,
        stack_base: Ptr,
        stack_size: libc::size_t,
        func:       extern fn(libc::intptr_t) -> !,
        data:       libc::intptr_t
    );
}
