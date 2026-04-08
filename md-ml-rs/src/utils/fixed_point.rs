// By Boshi Yuan (Rust rewrite)

use crate::share::ShareElement;

pub const FRACTION_BITS: usize = 16;
pub const TRUNCATE_VALUE: u64 = 1 << FRACTION_BITS;

#[inline]
pub fn truncate_clear<T: ShareElement>(x: T) -> T {
    x >> FRACTION_BITS
}

pub fn truncate_clear_vec<T: ShareElement>(x: &[T]) -> Vec<T> {
    x.iter().map(|&v| truncate_clear(v)).collect()
}

pub fn truncate_clear_vec_inplace<T: ShareElement>(x: &mut [T]) {
    for v in x.iter_mut() {
        *v = truncate_clear(*v);
    }
}

#[inline]
pub fn double2fix<T: ShareElement>(x: f64) -> T {
    T::from_f64(x * TRUNCATE_VALUE as f64)
}

pub fn double2fix_vec<T: ShareElement>(x: &[f64]) -> Vec<T> {
    x.iter().map(|&v| double2fix(v)).collect()
}

#[inline]
pub fn fix2double<T: ShareElement>(x: T) -> f64 {
    x.to_f64_signed() / TRUNCATE_VALUE as f64
}

pub fn fix2double_vec<T: ShareElement>(x: &[T]) -> Vec<f64> {
    x.iter().map(|&v| fix2double(v)).collect()
}
