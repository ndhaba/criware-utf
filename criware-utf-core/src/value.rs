use std::borrow::Cow;

/// All of the primitives that can be stored in a table
///
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ValueKind {
    U8 = 0,
    I8 = 1,
    U16 = 2,
    I16 = 3,
    U32 = 4,
    I32 = 5,
    U64 = 6,
    I64 = 7,
    F32 = 8,
    STR = 0xa,
    BLOB = 0xb,
}

pub(crate) mod sealed {
    use std::{borrow::Cow, collections::HashMap};

    #[doc(hidden)]
    pub trait Primitive: ToOwned {
        type Buffer: AsRef<[u8]> + AsMut<[u8]> + Default;

        const SIZE_IN_UTF: usize = std::mem::size_of::<Self::Buffer>();
        const TYPE_FLAG: super::ValueKind;

        fn parse<'a>(
            data: Self::Buffer,
            strings: &'a HashMap<u32, String>,
            blobs: &Vec<u8>,
        ) -> Option<Self::Owned>;

        fn write<'a>(
            value: Cow<'a, Self>,
            strings: &mut HashMap<Cow<'a, str>, u32>,
            string_buffer: &mut Vec<u8>,
            blobs: &mut Vec<u8>,
        ) -> Self::Buffer;
    }

    macro_rules! impl_primitive_number {
        ($($name:ident $flag:ident),+) => {
            $(
                impl Primitive for $name {
                    type Buffer = [u8; std::mem::size_of::<$name>()];

                    const TYPE_FLAG: super::ValueKind = super::ValueKind::$flag;

                    #[inline]
                    fn parse<'a>(
                        data: Self::Buffer,
                        _: &HashMap<u32, String>,
                        _: &Vec<u8>,
                    ) -> Option<Self> {
                        Some($name::from_be_bytes(data))
                    }
                    #[inline]
                    fn write<'a>(
                        value: Cow<'a, Self>,
                        _: &mut HashMap<Cow<'a, str>, u32>,
                        _: &mut Vec<u8>,
                        _: &mut Vec<u8>,
                    ) -> Self::Buffer {
                        value.to_be_bytes()
                    }
                }
            )*
        };
    }

    impl_primitive_number!(u8 U8, i8 I8, u16 U16, i16 I16, u32 U32, i32 I32, u64 U64, i64 I64, f32 F32);

    impl Primitive for str {
        type Buffer = [u8; 4];

        const TYPE_FLAG: super::ValueKind = super::ValueKind::STR;

        fn parse<'a>(
            data: Self::Buffer,
            strings: &HashMap<u32, String>,
            _: &Vec<u8>,
        ) -> Option<Self::Owned> {
            strings
                .get(&u32::from_be_bytes(data))
                .map(|v| v.to_string())
        }
        fn write<'a>(
            value: Cow<'a, Self>,
            strings: &mut HashMap<Cow<'a, str>, u32>,
            string_buffer: &mut Vec<u8>,
            _: &mut Vec<u8>,
        ) -> Self::Buffer {
            match strings.get(&value) {
                Some(idx) => (*idx).to_be_bytes(),
                None => {
                    let position = string_buffer.len() as u32;
                    string_buffer.extend_from_slice(&value.as_bytes());
                    string_buffer.push(0u8);
                    strings.insert(value, position);
                    position.to_be_bytes()
                }
            }
        }
    }

    impl Primitive for [u8] {
        type Buffer = [u8; 8];

        const TYPE_FLAG: super::ValueKind = super::ValueKind::BLOB;

        fn parse<'a>(
            data: Self::Buffer,
            _: &HashMap<u32, String>,
            blobs: &Vec<u8>,
        ) -> Option<Self::Owned> {
            let idx = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
            let len = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
            let end = idx + len;
            if end > blobs.len() {
                None
            } else {
                Some(blobs[idx..end].into())
            }
        }
        fn write<'a>(
            value: Cow<'a, Self>,
            _: &mut HashMap<Cow<'a, str>, u32>,
            _: &mut Vec<u8>,
            blobs: &mut Vec<u8>,
        ) -> Self::Buffer {
            let data = ((blobs.len() << 32) | value.len()) as u64;
            blobs.extend(value.iter());
            data.to_be_bytes()
        }
    }
}

macro_rules! blanket_impl {
    ($trait:ident for $($name:ty),+) => {
        $(
            impl $trait for $name {}
        )*
    };
}

/// A value that can be directly stored in a table (sealed)
///
pub trait Primitive: sealed::Primitive + ToOwned {}

blanket_impl!(Primitive for u8, u16, u32, u64, i8, i16, i32, i64, f32, str, [u8]);

/**
A value that can be stored in a table, but must be converted first

This trait allows for any arbitrary type to be encodable and decodable
from a UTF table, as long as it can be converted into one of the core
storeable types (implementors of trait [`Primitive`]).

# Example
```
# use std::{borrow::Cow, error::Error};
# use criware_utf_core::Value;
#[derive(Default)]
struct Buffer(String);

impl Value for Buffer {
    type Primitive = str;
    fn from_primitive(value: String) -> Result<Self, Box<dyn Error>> {
        Ok(Buffer(value))
    }
    fn to_primitive<'a>(&'a self) -> Result<Cow<'a, str>, Box<dyn Error>> {
        Ok(Cow::Borrowed(&self.0))
    }
}
```
*/
pub trait Value: Sized {
    /**
    The primitive to which this value will be converted to/from

    This may be [`u8`], [`i8`], [`u16`], [`i16`], [`u32`], [`i32`], [`u64`],
    [`i64`], [`f32`], [`str`], or `[u8]`
    */
    type Primitive: Primitive + ?Sized;

    /// Attempts to convert from the chosen primitive to this type.
    ///
    fn from_primitive(
        value: <Self::Primitive as ToOwned>::Owned,
    ) -> Result<Self, Box<dyn std::error::Error>>;

    /// Attempts to convert this value to the chosen primitive type.
    ///
    fn to_primitive<'a>(&'a self) -> Result<Cow<'a, Self::Primitive>, Box<dyn std::error::Error>>;
}

type BoxRes<T> = Result<T, Box<dyn std::error::Error>>;

macro_rules! impl_value_number {
    ($($type:ty),*) => {
        $(
            impl Value for $type {
                type Primitive = $type;
                #[inline]
                fn from_primitive(value: Self) -> BoxRes<Self> {
                    Ok(value)
                }
                #[inline]
                fn to_primitive<'a>(&'a self) -> BoxRes<Cow<'a, Self::Primitive>> {
                    Ok(Cow::Owned(*self))
                }
            }
        )*
    };
}

impl_value_number!(u8, u16, u32, u64, i8, i16, i32, i64, f32);

impl Value for String {
    type Primitive = str;

    #[inline]
    fn from_primitive(value: String) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(value)
    }
    #[inline]
    fn to_primitive<'a>(&'a self) -> Result<Cow<'a, Self::Primitive>, Box<dyn std::error::Error>> {
        Ok(Cow::Borrowed(self.as_str()))
    }
}

impl Value for Vec<u8> {
    type Primitive = [u8];

    #[inline]
    fn from_primitive(value: Vec<u8>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(value)
    }
    #[inline]
    fn to_primitive<'a>(&'a self) -> Result<Cow<'a, Self::Primitive>, Box<dyn std::error::Error>> {
        Ok(Cow::Borrowed(self.as_slice()))
    }
}

impl Value for Box<[u8]> {
    type Primitive = [u8];

    #[inline]
    fn from_primitive(value: Vec<u8>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(value.into_boxed_slice())
    }
    #[inline]
    fn to_primitive<'a>(&'a self) -> Result<Cow<'a, Self::Primitive>, Box<dyn std::error::Error>> {
        Ok(Cow::Borrowed(&self))
    }
}

impl<const N: usize> Value for [u8; N] {
    type Primitive = [u8];

    fn from_primitive(value: Vec<u8>) -> Result<Self, Box<dyn std::error::Error>> {
        match value.try_into() {
            Ok(value) => Ok(value),
            Err(_) => Err(crate::Error::BlobWrongSize.into()),
        }
    }
    #[inline]
    fn to_primitive<'a>(&'a self) -> Result<Cow<'a, Self::Primitive>, Box<dyn std::error::Error>> {
        Ok(Cow::Borrowed(self))
    }
}

/**
Returns the space (in bytes) a value would take up in column/row space

When a constant or rowed value is written, it is stored in the column or row
space (respectively). The space taken up in said space does not depend on the
data itself and can be determined at compile-time.

# Example
```
# extern crate criware_utf_core as criware_utf;
# use criware_utf::{Value, utf_size_of};
#[derive(Default)]
struct SplitU8(u8, u8);

impl Value for SplitU8 {
    type Primitive = u8;
    // ...
#   fn from_primitive(value: Self::Primitive) -> Result<Self, Box<dyn std::error::Error>> {
#       Ok(SplitU8(value >> 4, value & 15))
#   }
#   fn to_primitive<'a>(&'a self) -> Result<Cow<'a, Self::Primitive>, Box<dyn std::error::Error>> {
#       Ok(Cow::Owned((self.0 << 4) | self.1))
#   }
}

fn main() {
    assert_eq!(utf_size_of::<u8>(), 1);
    assert_eq!(utf_size_of::<i32>(), 4);
    assert_eq!(utf_size_of::<Vec<u8>>(), 8);
    assert_eq!(utf_size_of::<String>(), 4);
    assert_eq!(utf_size_of::<SplitU8>(), utf_size_of::<u8>());
}
```
*/
pub const fn utf_size_of<T: Value>() -> usize {
    <T::Primitive as sealed::Primitive>::SIZE_IN_UTF
}
