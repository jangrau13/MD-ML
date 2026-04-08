// By Boshi Yuan (Rust rewrite)

use std::num::Wrapping;

/// Trait representing a SPDZ-2k share type parameterized by K and S.
///
/// In the C++ version these are template parameters. In Rust we use a trait
/// with associated types and constants.
pub trait Spdz2kShare: 'static + Send + Sync {
    /// The number of bits for the value ring Z_{2^K}
    const K_BITS: usize;
    /// The number of bits for the MAC key ring Z_{2^S}
    const S_BITS: usize;

    /// Value type over Z_{2^K}
    type KType: ShareElement;
    /// MAC key share type over Z_{2^S}
    type SType: ShareElement;
    /// Combined type over Z_{2^{K+S}}
    type KSType: ShareElement;

    /// Clear value type (= KType)
    type ClearType: ShareElement;
    /// Semi-share type without MAC (= KSType)
    type SemiShrType: ShareElement;
    /// Key share type held by each party (= SType)
    type KeyShrType: ShareElement;
    /// Global key type used for MAC (= KSType)
    type GlobalKeyType: ShareElement;

    /// Remove upper S bits: Z_{2^{K+S}} -> Z_{2^K} (stored in KSType)
    fn remove_upper_bits(value: Self::SemiShrType) -> Self::SemiShrType;

    /// In-place removal of upper bits for a vector
    fn remove_upper_bits_inplace(values: &mut [Self::SemiShrType]) {
        for v in values.iter_mut() {
            *v = Self::remove_upper_bits(*v);
        }
    }
}

/// Trait for types that can be used as share elements (wrapping arithmetic).
pub trait ShareElement:
    Copy
    + Default
    + Send
    + Sync
    + 'static
    + std::fmt::Display
    + std::str::FromStr
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::BitXor<Output = Self>
    + std::ops::BitAnd<Output = Self>
    + std::ops::Shl<usize, Output = Self>
    + std::ops::Shr<usize, Output = Self>
    + Eq
    + Ord
{
    fn zero() -> Self;
    fn one() -> Self;
    fn wrapping_add(self, rhs: Self) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn wrapping_mul(self, rhs: Self) -> Self;
    fn random() -> Self;
    fn from_f64(v: f64) -> Self;
    fn to_f64_signed(self) -> f64;
    fn byte_size() -> usize;
    fn to_le_bytes_vec(self) -> Vec<u8>;
    fn from_le_bytes(bytes: &[u8]) -> Self;
    fn bit_count() -> usize;

    /// Zero-copy cast a slice of elements to a byte slice.
    /// On little-endian platforms this is a no-op pointer cast.
    fn slice_as_bytes(slice: &[Self]) -> &[u8];

    /// Zero-copy cast a byte slice back to a slice of elements.
    /// The byte slice length must be a multiple of `byte_size()`.
    /// On little-endian platforms this is a no-op pointer cast.
    fn slice_from_bytes(bytes: &[u8]) -> &[Self];

    /// Copy bytes into a new Vec of elements.
    fn vec_from_bytes(bytes: &[u8]) -> Vec<Self>;

    /// Fill a pre-allocated slice of elements with random values using the given RNG.
    fn fill_random(slice: &mut [Self]);
}

macro_rules! impl_share_element {
    ($t:ty, $size:expr) => {
        impl ShareElement for $t {
            #[inline]
            fn zero() -> Self { 0 }
            #[inline]
            fn one() -> Self { 1 }
            #[inline]
            fn wrapping_add(self, rhs: Self) -> Self { (Wrapping(self) + Wrapping(rhs)).0 }
            #[inline]
            fn wrapping_sub(self, rhs: Self) -> Self { (Wrapping(self) - Wrapping(rhs)).0 }
            #[inline]
            fn wrapping_mul(self, rhs: Self) -> Self { (Wrapping(self) * Wrapping(rhs)).0 }
            fn random() -> Self {
                use rand::Rng;
                rand::thread_rng().gen()
            }
            #[inline]
            fn from_f64(v: f64) -> Self { v as Self }
            #[inline]
            fn to_f64_signed(self) -> f64 { (self as <$t as SignedCounterpart>::Signed) as f64 }
            #[inline]
            fn byte_size() -> usize { $size }
            #[inline]
            fn to_le_bytes_vec(self) -> Vec<u8> { self.to_le_bytes().to_vec() }
            #[inline]
            fn from_le_bytes(bytes: &[u8]) -> Self {
                let mut arr = [0u8; $size];
                arr.copy_from_slice(&bytes[..$size]);
                <$t>::from_le_bytes(arr)
            }
            #[inline]
            fn bit_count() -> usize { $size * 8 }

            #[inline]
            fn slice_as_bytes(slice: &[Self]) -> &[u8] {
                // Safety: $t has no padding and little-endian byte order matches
                // the memory layout on little-endian platforms (x86, ARM).
                unsafe {
                    std::slice::from_raw_parts(
                        slice.as_ptr() as *const u8,
                        slice.len() * $size,
                    )
                }
            }

            #[inline]
            fn slice_from_bytes(bytes: &[u8]) -> &[Self] {
                assert!(bytes.len() % $size == 0);
                // Safety: same as above, alignment is 1 for reading
                unsafe {
                    std::slice::from_raw_parts(
                        bytes.as_ptr() as *const $t,
                        bytes.len() / $size,
                    )
                }
            }

            fn vec_from_bytes(bytes: &[u8]) -> Vec<Self> {
                assert!(bytes.len() % $size == 0);
                let count = bytes.len() / $size;
                let mut result = Vec::with_capacity(count);
                // Safety: we're copying bytes into properly-sized elements
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        result.as_mut_ptr() as *mut u8,
                        bytes.len(),
                    );
                    result.set_len(count);
                }
                result
            }

            fn fill_random(slice: &mut [Self]) {
                use rand::RngCore;
                // Fill the slice as bytes directly — much faster than per-element gen()
                let byte_slice = unsafe {
                    std::slice::from_raw_parts_mut(
                        slice.as_mut_ptr() as *mut u8,
                        slice.len() * $size,
                    )
                };
                rand::thread_rng().fill_bytes(byte_slice);
            }
        }
    };
}

trait SignedCounterpart { type Signed; }
impl SignedCounterpart for u32 { type Signed = i32; }
impl SignedCounterpart for u64 { type Signed = i64; }
impl SignedCounterpart for u128 { type Signed = i128; }

impl_share_element!(u32, 4);
impl_share_element!(u64, 8);
impl_share_element!(u128, 16);

/// Widen a smaller integer type to a larger one via little-endian bytes.
#[inline]
pub fn widen<From: ShareElement, To: ShareElement>(val: From) -> To {
    let bytes = val.to_le_bytes_vec();
    let mut extended = [0u8; 16];
    extended[..bytes.len()].copy_from_slice(&bytes);
    To::from_le_bytes(&extended[..To::byte_size()])
}

// --- Concrete Share Types ---

/// Spdz2kShare<32, 32> using u32 for values and u64 for combined
pub struct Spdz2kShare32;

impl Spdz2kShare for Spdz2kShare32 {
    const K_BITS: usize = 32;
    const S_BITS: usize = 32;

    type KType = u32;
    type SType = u32;
    type KSType = u64;
    type ClearType = u32;
    type SemiShrType = u64;
    type KeyShrType = u32;
    type GlobalKeyType = u64;

    #[inline]
    fn remove_upper_bits(value: u64) -> u64 {
        (value << 32) >> 32
    }
}

/// Spdz2kShare<64, 64> using u64 for values and u128 for combined
pub struct Spdz2kShare64;

impl Spdz2kShare for Spdz2kShare64 {
    const K_BITS: usize = 64;
    const S_BITS: usize = 64;

    type KType = u64;
    type SType = u64;
    type KSType = u128;
    type ClearType = u64;
    type SemiShrType = u128;
    type KeyShrType = u64;
    type GlobalKeyType = u128;

    #[inline]
    fn remove_upper_bits(value: u128) -> u128 {
        (value << 64) >> 64
    }
}
