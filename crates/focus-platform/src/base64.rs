//! Standard base64 (RFC 4648, `A-Z a-z 0-9 + /`, `=` padding) for raw registry values in restore files, and the
//! URL-safe unpadded variant (`- _`, RFC 4648 §5) for OAuth PKCE values and JWT payloads.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// URL-safe base64 without padding (`base64url`, as PKCE and JWT use it).
pub(crate) fn encode_url(bytes: &[u8]) -> String {
    encode(bytes)
        .trim_end_matches('=')
        .chars()
        .map(|c| match c {
            '+' => '-',
            '/' => '_',
            other => other,
        })
        .collect()
}

/// Decodes unpadded (or padded) URL-safe base64; `None` for anything else.
pub(crate) fn decode_url(text: &str) -> Option<Vec<u8>> {
    if text.contains(['+', '/']) {
        return None;
    }
    let mut standard: String = text
        .trim_end_matches('=')
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();
    match standard.len() % 4 {
        0 => {}
        2 => standard.push_str("=="),
        3 => standard.push('='),
        _ => return None,
    }
    decode(&standard)
}

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        for (index, shift) in [18u32, 12, 6, 0].into_iter().enumerate() {
            if index <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> shift) & 0x3F) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn sextet(byte: u8) -> Option<u32> {
    let value = match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(u32::from(value))
}

/// Strict decoding: no whitespace, correct padding, zero unused bits. `None` for anything else.
pub(crate) fn decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let chunks = bytes.len() / 4;
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index + 1 == chunks;
        let padding = chunk.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for &byte in &chunk[..4 - padding] {
            n = (n << 6) | sextet(byte)?;
        }
        n <<= 6 * padding as u32;
        let decoded = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        let keep = 3 - padding;
        // Bits that do not fit into the kept bytes must be zero (canonical encoding).
        if decoded[keep..].iter().any(|&byte| byte != 0) {
            return None;
        }
        out.extend_from_slice(&decoded[..keep]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{decode, decode_url, encode, encode_url};

    #[test]
    fn url_safe_variant() {
        assert_eq!(encode_url(&[0xfb, 0xff]), "-_8");
        assert_eq!(encode_url(b"foob"), "Zm9vYg");
        assert_eq!(decode_url("-_8").as_deref(), Some(&[0xfb, 0xff][..]));
        assert_eq!(decode_url("Zm9vYg").as_deref(), Some(&b"foob"[..]));
        assert_eq!(decode_url("Zm9vYg==").as_deref(), Some(&b"foob"[..]));
        assert_eq!(decode_url("").as_deref(), Some(&b""[..]));
        assert_eq!(decode_url("Z"), None);
        assert_eq!(decode_url("+/8"), None);
        let bytes: Vec<u8> = (0..=255u8).collect();
        for len in 0..bytes.len() {
            assert_eq!(
                decode_url(&encode_url(&bytes[..len])).as_deref(),
                Some(&bytes[..len])
            );
        }
    }

    #[test]
    fn rfc4648_vectors() {
        let cases: [(&[u8], &str); 7] = [
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
        ];
        for (raw, text) in cases {
            assert_eq!(encode(raw), text);
            assert_eq!(decode(text).as_deref(), Some(raw));
        }
    }

    #[test]
    fn binary_round_trip() {
        let bytes: Vec<u8> = (0..=255u8).chain((0..=255u8).rev()).collect();
        for len in 0..bytes.len() {
            assert_eq!(
                decode(&encode(&bytes[..len])).as_deref(),
                Some(&bytes[..len])
            );
        }
    }

    #[test]
    fn rejects_malformed_text() {
        for text in [
            "Zg", "Zg=", "Z===", "Zg==Zg==", "Zm9v\n", "Zm9*", "Zh==", " Zm9v",
        ] {
            assert_eq!(decode(text), None, "{text:?}");
        }
    }
}
