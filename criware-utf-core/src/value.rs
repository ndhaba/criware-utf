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
    use std::{collections::HashMap, hint::unreachable_unchecked, mem::transmute};

    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum StorageMethod {
        Number,
        String,
        Blob,
    }

    #[doc(hidden)]
    pub trait Primitive: Sized + Default {
        type Buffer: AsRef<[u8]> + AsMut<[u8]> + Default;

        const SIZE_IN_UTF: usize = std::mem::size_of::<Self::Buffer>();
        const STORAGE_TYPE: StorageMethod;
        const TYPE_FLAG: super::ValueKind;

        fn parse_number(_data: Self::Buffer) -> Self {
            unsafe { unreachable_unchecked() }
        }
        fn parse_string(_data: Self::Buffer, _strs: &HashMap<u32, String>) -> Option<Self> {
            unsafe { unreachable_unchecked() }
        }
        fn parse_blob(_data: Self::Buffer, _blob: &Vec<u8>) -> Option<Self> {
            unsafe { unreachable_unchecked() }
        }
        fn write_number(&self) -> Self::Buffer {
            unsafe { unreachable_unchecked() }
        }
        fn write_string(
            &self,
            _strings: &mut HashMap<String, u32>,
            _string_buffer: &mut Vec<u8>,
        ) -> Self::Buffer {
            unsafe { unreachable_unchecked() }
        }
        fn write_blob(&self, _blobs: &mut Vec<u8>) -> Self::Buffer {
            unsafe { unreachable_unchecked() }
        }
    }

    macro_rules! impl_primitive_number {
        ($($name:ident $flag:ident),+) => {
            $(
                impl Primitive for $name {
                    type Buffer = [u8; std::mem::size_of::<$name>()];

                    const STORAGE_TYPE: StorageMethod = StorageMethod::Number;
                    const TYPE_FLAG: super::ValueKind = super::ValueKind::$flag;

                    #[inline(always)]
                    fn parse_number(data: Self::Buffer) -> Self {
                        $name::from_be_bytes(data)
                    }
                    #[inline(always)]
                    fn write_number(&self) -> Self::Buffer {
                        self.to_be_bytes()
                    }
                }
            )*
        };
    }

    impl_primitive_number!(u8 U8, i8 I8, u16 U16, i16 I16, u32 U32, i32 I32, u64 U64, i64 I64, f32 F32);

    impl Primitive for String {
        type Buffer = [u8; 4];

        const STORAGE_TYPE: StorageMethod = StorageMethod::String;
        const TYPE_FLAG: super::ValueKind = super::ValueKind::STR;

        #[inline(always)]
        fn parse_string(data: Self::Buffer, strings: &HashMap<u32, String>) -> Option<Self> {
            strings.get(&u32::from_be_bytes(data)).map(|v| v.clone())
        }
        fn write_string(
            &self,
            strings: &mut HashMap<String, u32>,
            string_buffer: &mut Vec<u8>,
        ) -> Self::Buffer {
            match strings.get(self) {
                Some(idx) => (*idx).to_be_bytes(),
                None => {
                    let position = string_buffer.len() as u32;
                    string_buffer.extend_from_slice(self.as_bytes());
                    string_buffer.push(0u8);
                    strings.insert(self.clone(), position);
                    position.to_be_bytes()
                }
            }
        }
    }

    impl Primitive for Vec<u8> {
        type Buffer = [u8; 8];

        const STORAGE_TYPE: StorageMethod = StorageMethod::Blob;
        const TYPE_FLAG: super::ValueKind = super::ValueKind::BLOB;

        fn parse_blob(data: Self::Buffer, blob: &Vec<u8>) -> Option<Self> {
            // This is completely safe
            // This splits the [u8; 8] into a left and right [u8; 4],
            // which works on any platform regardless of endianness
            let (b1, b2): ([u8; 4], [u8; 4]) = unsafe { transmute(data) };
            // safe code
            let idx = u32::from_be_bytes(b1) as usize;
            let len = u32::from_be_bytes(b2) as usize;
            let end = idx + len;
            if end > blob.len() {
                None
            } else {
                Some(blob[idx..end].into())
            }
        }
        fn write_blob(&self, data: &mut Vec<u8>) -> Self::Buffer {
            let idx = (data.len() as u32).to_be();
            let len = (self.len() as u32).to_be();
            data.extend(self);
            unsafe { transmute((idx, len)) }
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

pub trait Primitive: sealed::Primitive {}

blanket_impl!(Primitive for u8, u16, u32, u64, i8, i16, i32, i64, f32, String, Vec<u8>);

pub trait Value: Sized + Default {
    type Primitive: Primitive;

    fn from_utf_value(value: Self::Primitive) -> Result<Self, Box<dyn std::error::Error>>;
    fn to_utf_value<'a>(&'a self) -> Result<&'a Self::Primitive, Box<dyn std::error::Error>>;
}

impl<T: Primitive> Value for T {
    type Primitive = T;

    #[inline(always)]
    fn from_utf_value(value: Self::Primitive) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(value)
    }
    #[inline(always)]
    fn to_utf_value<'a>(&'a self) -> Result<&'a Self::Primitive, Box<dyn std::error::Error>> {
        Ok(&self)
    }
}

#[inline(always)]
pub const fn utf_size_of<T: Value>() -> usize {
    <T::Primitive as sealed::Primitive>::SIZE_IN_UTF
}
