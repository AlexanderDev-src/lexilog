//! Chart images attached to writing pieces (Task 1).
//!
//! The browser crops and shrinks the image before upload, but the server
//! still checks what it receives: the format comes from the file's first
//! bytes, never from the Content-Type header, and oversized images are
//! refused so one upload can't eat the AI token quota.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::error::{AppError, AppResult};

/// Largest upload accepted. A 1280 px chart as PNG is usually far smaller.
pub const MAX_IMAGE_BYTES: usize = 3 * 1024 * 1024;
/// Longest side accepted. The browser sends at most 1280 px.
pub const MAX_IMAGE_SIDE: u32 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Webp,
}

impl ImageFormat {
    pub fn mime(self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Webp => "image/webp",
        }
    }
}

/// What the server learned from the image's header bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageMeta {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}

/// Describes a stored image, without its bytes (sent inside `Piece`).
#[derive(Debug, Clone, Serialize)]
pub struct ImageInfo {
    pub mime: String,
    pub width: i64,
    pub height: i64,
    pub bytes: i64,
    pub created_at: DateTime<Utc>,
}

/// A stored image with its bytes, for serving it or sending it to the AI.
#[derive(Debug, Clone)]
pub struct StoredImage {
    pub mime: String,
    pub data: Vec<u8>,
}

/// Reads the format and size from the header and checks the limits.
pub fn inspect(bytes: &[u8]) -> AppResult<ImageMeta> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::Validation(format!(
            "the image is too large ({} KB); the limit is {} KB",
            bytes.len() / 1024,
            MAX_IMAGE_BYTES / 1024
        )));
    }
    let meta = png_size(bytes)
        .or_else(|| webp_size(bytes))
        .ok_or_else(|| AppError::Validation("the image must be a PNG or WebP file".into()))?;
    if meta.width == 0 || meta.height == 0 {
        return Err(AppError::Validation("the image has no pixels".into()));
    }
    if meta.width.max(meta.height) > MAX_IMAGE_SIDE {
        return Err(AppError::Validation(format!(
            "the image is {}×{} px; the longest side can be at most {MAX_IMAGE_SIDE} px",
            meta.width, meta.height
        )));
    }
    Ok(meta)
}

/// PNG: an 8-byte signature, then the IHDR chunk with width and height as
/// big-endian 32-bit numbers at bytes 16..20 and 20..24.
fn png_size(b: &[u8]) -> Option<ImageMeta> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if b.len() < 24 || !b.starts_with(SIGNATURE) || &b[12..16] != b"IHDR" {
        return None;
    }
    Some(ImageMeta {
        format: ImageFormat::Png,
        width: u32::from_be_bytes(b[16..20].try_into().ok()?),
        height: u32::from_be_bytes(b[20..24].try_into().ok()?),
    })
}

/// WebP: "RIFF", a length, "WEBP", then one of three chunk kinds. Each stores
/// the size differently (all little-endian):
/// - VP8X (extended): width-1 and height-1 as 24-bit numbers at 24 and 27
/// - VP8L (lossless): 14 bits each, packed after a 0x2f byte at 20
/// - "VP8 " (lossy): 14 bits each at 26 and 28, after a 3-byte start code
fn webp_size(b: &[u8]) -> Option<ImageMeta> {
    if b.len() < 30 || &b[0..4] != b"RIFF" || &b[8..12] != b"WEBP" {
        return None;
    }
    let u24 = |i: usize| u32::from(b[i]) | u32::from(b[i + 1]) << 8 | u32::from(b[i + 2]) << 16;
    let u16le = |i: usize| u32::from(b[i]) | u32::from(b[i + 1]) << 8;
    let (width, height) = match &b[12..16] {
        b"VP8X" => (u24(24) + 1, u24(27) + 1),
        b"VP8L" if b[20] == 0x2f => {
            let bits = u32::from_le_bytes(b[21..25].try_into().ok()?);
            ((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1)
        }
        b"VP8 " if b[23..26] == [0x9d, 0x01, 0x2a] => (u16le(26) & 0x3fff, u16le(28) & 0x3fff),
        _ => return None,
    };
    Some(ImageMeta {
        format: ImageFormat::Webp,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        b.extend(width.to_be_bytes());
        b.extend(height.to_be_bytes());
        b.extend([8, 6, 0, 0, 0]);
        b
    }

    fn webp(chunk: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut b = b"RIFF\0\0\0\0WEBP".to_vec();
        b.extend(chunk);
        b.extend([0; 4]); // chunk length, not read
        b.extend(payload);
        b.resize(40, 0);
        b
    }

    #[test]
    fn reads_png_size() {
        let meta = inspect(&png(1024, 683)).unwrap();
        assert_eq!(meta.format, ImageFormat::Png);
        assert_eq!((meta.width, meta.height), (1024, 683));
    }

    #[test]
    fn reads_all_three_webp_kinds() {
        // VP8X: flags + 3 reserved bytes, then width-1 and height-1 (24-bit).
        let vp8x = webp(b"VP8X", &[0, 0, 0, 0, 0xff, 0x03, 0, 0xaa, 0x02, 0]);
        let meta = inspect(&vp8x).unwrap();
        assert_eq!(
            (meta.format, meta.width, meta.height),
            (ImageFormat::Webp, 1024, 683)
        );

        // VP8L: 0x2f, then (width-1) | (height-1) << 14 in 4 bytes.
        let bits: u32 = 1023 | 682 << 14;
        let mut payload = vec![0x2f];
        payload.extend(bits.to_le_bytes());
        let meta = inspect(&webp(b"VP8L", &payload)).unwrap();
        assert_eq!((meta.width, meta.height), (1024, 683));

        // "VP8 ": 3-byte frame tag, start code, then width and height (16-bit).
        let mut payload = vec![0, 0, 0, 0x9d, 0x01, 0x2a];
        payload.extend(1024u16.to_le_bytes());
        payload.extend(683u16.to_le_bytes());
        let meta = inspect(&webp(b"VP8 ", &payload)).unwrap();
        assert_eq!((meta.width, meta.height), (1024, 683));
    }

    #[test]
    fn rejects_other_files_and_oversized_images() {
        assert!(inspect(b"GIF89a....................................").is_err());
        assert!(inspect(b"\xff\xd8\xff\xe0 a jpeg is not accepted.......").is_err());
        assert!(inspect(&png(4000, 3000)).is_err());
        assert!(inspect(&png(0, 10)).is_err());
        let mut huge = png(100, 100);
        huge.resize(MAX_IMAGE_BYTES + 1, 0);
        assert!(inspect(&huge).is_err());
    }
}
