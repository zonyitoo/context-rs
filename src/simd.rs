// Temporary

#[repr(simd)]
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct u32x4(u32, u32, u32, u32);

impl u32x4 {
    pub fn new(a: u32, b: u32, c: u32, d: u32) -> u32x4 {
        u32x4(a, b, c, d)
    }
}
