pub(crate) mod sealed {
    use std::collections::HashMap;

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum UTFValueStorageMethod {
        Number,
        String,
        Blob,
    }

    #[doc(hidden)]
    pub trait Primitive: Sized {
        type Buffer: AsMut<[u8]> + Default;

        const SIZE_IN_UTF: usize = std::mem::size_of::<Self::Buffer>();
        const STORAGE_TYPE: UTFValueStorageMethod;
        const TYPE_FLAG: u8;

        unsafe fn parse_number(_data: Self::Buffer) -> Self {
            unsafe { std::hint::unreachable_unchecked() }
        }
        unsafe fn parse_string(
            _data: Self::Buffer,
            _strings: &HashMap<u32, String>,
        ) -> Option<Self> {
            unsafe { std::hint::unreachable_unchecked() }
        }
        unsafe fn parse_blob(_data: Self::Buffer, _blob: &Vec<u8>) -> Option<Self> {
            unsafe { std::hint::unreachable_unchecked() }
        }
    }

    macro_rules! impl_primitive_number {
        ($($name:ident $flag:expr),+) => {
            $(
                impl Primitive for $name {
                    type Buffer = [u8; std::mem::size_of::<$name>()];

                    const STORAGE_TYPE: UTFValueStorageMethod = UTFValueStorageMethod::Number;
                    const TYPE_FLAG: u8 = $flag;

                    #[inline(always)]
                    unsafe fn parse_number(data: Self::Buffer) -> Self {
                        $name::from_be_bytes(data)
                    }
                }
            )*
        };
    }

    impl_primitive_number!(u8 0, i8 1, u16 2, i16 3, u32 4, i32 5, u64 6, i64 7, f32 8);

    impl Primitive for String {
        type Buffer = [u8; 4];

        const STORAGE_TYPE: UTFValueStorageMethod = UTFValueStorageMethod::String;
        const TYPE_FLAG: u8 = 0xa;

        #[inline(always)]
        unsafe fn parse_string(data: Self::Buffer, strings: &HashMap<u32, String>) -> Option<Self> {
            strings.get(&u32::from_be_bytes(data)).map(|v| v.clone())
        }
    }

    impl Primitive for Vec<u8> {
        type Buffer = [u8; 8];

        const STORAGE_TYPE: UTFValueStorageMethod = UTFValueStorageMethod::Blob;
        const TYPE_FLAG: u8 = 0xb;

        unsafe fn parse_blob(data: Self::Buffer, blob: &Vec<u8>) -> Option<Self> {
            // This is completely safe
            // This splits the [u8; 8] into a left and right [u8; 4],
            // which works on any platform regardless of endianness
            let (b1, b2): ([u8; 4], [u8; 4]) = unsafe { std::mem::transmute(data) };
            // safe code
            let idx = u32::from_be_bytes(b1) as usize;
            let len = u32::from_be_bytes(b2) as usize;
            let end = idx + len;
            if end >= blob.len() {
                None
            } else {
                Some(blob[idx..end].into())
            }
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

pub trait UTFPrimitive: sealed::Primitive {}

blanket_impl!(UTFPrimitive for u8, u16, u32, u64, i8, i16, i32, i64, f32, String, Vec<u8>);

pub trait UTFValue: Sized {
    type Primitive: UTFPrimitive;

    fn from_utf_value(value: Self::Primitive) -> Result<Self, Box<dyn std::error::Error>>;
    fn to_utf_value(self) -> Self::Primitive;
}

impl<T: UTFPrimitive> UTFValue for T {
    type Primitive = T;

    #[inline(always)]
    fn from_utf_value(value: Self::Primitive) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(value)
    }
    #[inline(always)]
    fn to_utf_value(self) -> Self::Primitive {
        self
    }
}

#[inline(always)]
pub const fn utf_size_of<T: UTFValue>() -> usize {
    <T::Primitive as sealed::Primitive>::SIZE_IN_UTF
}
