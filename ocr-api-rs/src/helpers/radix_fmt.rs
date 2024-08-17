use core::num::{NonZeroI128, NonZeroU128};
use core::num::{NonZeroI16, NonZeroU16};
use core::num::{NonZeroI32, NonZeroU32};
use core::num::{NonZeroI64, NonZeroU64};
use core::num::{NonZeroI8, NonZeroU8};
use core::num::{NonZeroIsize, NonZeroUsize};
use std::fmt::{Display, Formatter, Result};
use std::iter::successors;

pub type Base = u8;

pub const MIN_BASE: Base = 2;
pub const MAX_BASE: Base = 36;

/// A struct to format a number in an arbitrary radix.
///
/// # Example:
///
/// ```rust
/// use radix_fmt::Radix;
/// use std::fmt::Write;
///
/// let n = 15;
/// let mut s = String::new();
///
/// write!(&mut s, "{}", Radix::new(n, 3));
/// assert_eq!(s, "120"); // 15 is "120" in base 3
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Radix<T> {
    n: T,
    base: Base,
}

impl<T> Radix<T>
where
    Self: Display,
{
    /// Create a new displayable number from number and base.
    /// The base must be in the range [2, 36].
    pub fn new(n: T, base: Base) -> Self {
        assert!((MIN_BASE..=MAX_BASE).contains(&base));

        Self { n, base }
    }
}

fn digit(u: Base, alternate: bool) -> Base {
    let a = if alternate { b'A' } else { b'a' };

    match u {
        0..=9 => u + b'0',
        10..=35 => u - 10 + a,
        _ => unreachable!("Digit is not in range [0..36]"),
    }
}

const BUF_SIZE: usize = 81; // u128::max_value() in base 3 takes 81 digits to write.

pub trait FormatRadix {
    fn format_to_base(self, radix: Base) -> String;
}

macro_rules! impl_display_for {
    ($i: ty => $via: ty as $u: ty) => {
        impl Display for Radix<$i> {
            fn fmt(&self, f: &mut Formatter) -> Result {
                fn do_format(n: $u, base: $u, f: &mut Formatter) -> Result {
                    let mut buffer = [0_u8; BUF_SIZE];
                    let divided = successors(Some(n), |n| match n / base {
                        0 => None,
                        n => Some(n),
                    });
                    #[allow(clippy::cast_possible_truncation)]
                    let written = buffer
                        .iter_mut()
                        .rev()
                        .zip(divided)
                        .map(|(c, n)| *c = digit((n % base) as Base, f.alternate()))
                        .count();
                    let index = BUF_SIZE - written;

                    // There are only ASCII chars inside the buffer, so the string
                    // is guaranteed to be a valid UTF-8 string.
                    let s = unsafe { std::str::from_utf8_unchecked(&buffer[index..]) };

                    f.write_str(s)
                }

                match (self.base, f.alternate()) {
                    (2, _) => write!(f, "{:b}", self.n),
                    (8, _) => write!(f, "{:o}", self.n),
                    (10, _) => write!(f, "{}", self.n),
                    (16, false) => write!(f, "{:x}", self.n),
                    (16, true) => write!(f, "{:X}", self.n),
                    #[allow(clippy::cast_sign_loss)]
                    (base, _) => do_format(<$via>::from(self.n) as $u, base.into(), f),
                }
            }
        }

        impl FormatRadix for $i {
            fn format_to_base(self, radix: Base) -> String {
                Radix::new(self, radix).to_string()
            }
        }
    };
}

impl_display_for!(i8 => i8 as u8);
impl_display_for!(u8 => u8 as u8);

impl_display_for!(i16 => i16 as u16);
impl_display_for!(u16 => u16 as u16);

impl_display_for!(i32 => i32 as u32);
impl_display_for!(u32 => u32 as u32);

impl_display_for!(i64 => i64 as u64);
impl_display_for!(u64 => u64 as u64);

impl_display_for!(i128 => i128 as u128);
impl_display_for!(u128 => u128 as u128);

impl_display_for!(isize => isize as usize);
impl_display_for!(usize => usize as usize);

impl_display_for!(NonZeroI8 => i8 as u8);
impl_display_for!(NonZeroU8 => u8 as u8);

impl_display_for!(NonZeroI16 => i16 as u16);
impl_display_for!(NonZeroU16 => u16 as u16);

impl_display_for!(NonZeroI32 => i32 as u32);
impl_display_for!(NonZeroU32 => u32 as u32);

impl_display_for!(NonZeroI64 => i64 as u64);
impl_display_for!(NonZeroU64 => u64 as u64);

impl_display_for!(NonZeroI128 => i128 as u128);
impl_display_for!(NonZeroU128 => u128 as u128);

impl_display_for!(NonZeroIsize => isize as usize);
impl_display_for!(NonZeroUsize => usize as usize);
