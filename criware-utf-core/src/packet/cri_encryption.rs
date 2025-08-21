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

fn decrypt_fallback_raw(src: &[u8], dst: &mut [u8], mut i: usize) {
    let mask = DECRYPTION_MASK;
    while i < src.len() {
        dst[i] = src[i] ^ mask[i & 63];
        i += 1;
    }
}

fn decrypt_fallback(src: &[u8], dst: &mut [u8]) {
    decrypt_fallback_raw(src, dst, 0);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
fn decrypt_sse2(src: &[u8], dst: &mut [u8]) {
    let end = src.len() - (src.len() & 0xf);
    let count = src.len().wrapping_shr(4);
    let mut i = 0usize;
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{__m128i, _mm_xor_si128};
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{__m128i, _mm_xor_si128};
        let mask: [__m128i; 4] = transmute(DECRYPTION_MASK);
        let src: &[__m128i] = transmute(&(src[0..end]));
        let dst: &mut [__m128i] = transmute(&mut (dst[0..end]));
        while i < count {
            dst[i] = _mm_xor_si128(src[i], mask[i & 3]);
            i += 1;
        }
        i <<= 4;
    };
    decrypt_fallback_raw(&src[end..], &mut dst[end..], i);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
fn decrypt_avx2(src: &[u8], dst: &mut [u8]) {
    let end = src.len() - (src.len() & 0x1f);
    let count = src.len().wrapping_shr(5);
    let mut i = 0usize;
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{__m256i, _mm256_xor_si256};
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{__m256i, _mm256_xor_si256};
        let mask: (__m256i, __m256i) = transmute(DECRYPTION_MASK);
        let src: &[__m256i] = transmute(src);
        let dst: &mut [__m256i] = transmute(&mut *dst);
        while i < count - 1 {
            dst[i..i + 2].copy_from_slice(&[
                _mm256_xor_si256(*src.get_unchecked(i), mask.0),
                _mm256_xor_si256(*src.get_unchecked(i + 1), mask.1),
            ]);
            i += 2;
        }
        if i < count {
            dst[i] = _mm256_xor_si256(src[i], mask.0);
            i += 1;
        }
        i <<= 5;
    };
    decrypt_fallback_raw(&src[end..], &mut dst[end..], i);
}

pub fn decrypt(src: &[u8], dst: &mut [u8]) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe {
        if is_x86_feature_detected!("avx2") {
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
