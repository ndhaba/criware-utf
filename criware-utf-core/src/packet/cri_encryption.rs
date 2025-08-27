use std::mem::transmute;

static DECRYPTION_MASK: [u8; 64] = [
    95u8, 203u8, 167u8, 179u8, 175u8, 91u8, 119u8, 195u8, 255u8, 235u8, 71u8, 211u8, 79u8, 123u8,
    23u8, 227u8, 159u8, 11u8, 231u8, 243u8, 239u8, 155u8, 183u8, 3u8, 63u8, 43u8, 135u8, 19u8,
    143u8, 187u8, 87u8, 35u8, 223u8, 75u8, 39u8, 51u8, 47u8, 219u8, 247u8, 67u8, 127u8, 107u8,
    199u8, 83u8, 207u8, 251u8, 151u8, 99u8, 31u8, 139u8, 103u8, 115u8, 111u8, 27u8, 55u8, 131u8,
    191u8, 171u8, 7u8, 147u8, 15u8, 59u8, 215u8, 163u8,
];

pub fn can_decrypt(src: &[u8]) -> bool {
    u32::from_le_bytes(src[0..4].try_into().unwrap()) == 0xF5F39E1Fu32
}

pub fn decrypt_fallback(src: &[u8], dst: &mut [u8]) {
    let count = src.len().div_ceil(8);
    let mut i = 0usize;
    unsafe {
        let mask: [u64; 8] = transmute(DECRYPTION_MASK);
        let src: &[u64] = transmute(src);
        let dst: &mut [u64] = transmute(&mut *dst);
        while i < count {
            dst[i] = src[i] ^ mask[i & 7];
            i += 1;
        }
    };
}

macro_rules! decrypt_vectored {
    {
        src = $src:expr,
        dst = $dst:expr,
        vector_type = $ty:ty,
        vector_xor = $func:ident,
        vector_bits = $bits:literal
    } => {
        const IDX_MASK: usize = (512 / $bits) - 1;
        let count = $src.len().div_ceil($bits >> 3);
        let mut i = 0usize;
        // direct SIMD instructions are always unsafe
        unsafe {
            #[cfg(target_arch = "x86")]
            use std::arch::x86::{$func, $ty};
            #[cfg(target_arch = "x86_64")]
            use std::arch::x86_64::{$func, $ty};

            let mask: [$ty; (512 / $bits)] = transmute(DECRYPTION_MASK);
            let src: &[$ty] = transmute($src);
            let dst: &mut [$ty] = transmute(&mut *$dst);
            while i < count {
                dst[i] = $func(src[i], mask[i & IDX_MASK]);
                i += 1;
            }
        }
    };
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
fn decrypt_sse2(src: &[u8], dst: &mut [u8]) {
    decrypt_vectored! {
        src = src,
        dst = dst,
        vector_type = __m128i,
        vector_xor = _mm_xor_si128,
        vector_bits = 128
    };
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
fn decrypt_avx2(src: &[u8], dst: &mut [u8]) {
    decrypt_vectored! {
        src = src,
        dst = dst,
        vector_type = __m256i,
        vector_xor = _mm256_xor_si256,
        vector_bits = 256
    };
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
fn decrypt_avx512f(src: &[u8], dst: &mut [u8]) {
    decrypt_vectored! {
        src = src,
        dst = dst,
        vector_type = __m512i,
        vector_xor = _mm512_xor_si512,
        vector_bits = 512
    };
}

pub fn decrypt(src: &[u8], dst: &mut [u8]) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe {
        if is_x86_feature_detected!("avx512f") {
            // untested :(
            return decrypt_avx512f(src, dst);
        } else if is_x86_feature_detected!("avx2") {
            return decrypt_avx2(src, dst);
        } else if is_x86_feature_detected!("sse2") {
            return decrypt_sse2(src, dst);
        }
    }
    decrypt_fallback(src, dst);
}

pub fn encrypt(src: &[u8], dst: &mut [u8]) {
    decrypt(src, dst);
}
