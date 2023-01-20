use serde::{Serialize, Deserialize};

use crate::error::{Result, ErrorKind, Error};

pub const BYTES_PER_PIXEL: u32 = 4;
pub const B2B_SIGNATURE: u128 = 0x6FAFEC0D7EF10C4468E85B0B9C0FB9E;
pub const BITMAP_HEADER_SIZE: u32 = 0x8A;
pub const BITMAP_ID: u16 = 0x4D42;
pub const B2B_HEADER_SIZE: u32 = 40;

#[derive(Serialize, Deserialize)]
struct BitmapV5Header {
    //BMP Header
    id: u16,
    file_size: u32,
    unused1: u32,
    offset: u32,

    //DIB Header
    dib_size: u32,
    width: u32,
    height: u32,
    pbnlanes: u16,
    bpp: u16,
    compression: u32,
    pixmap_size: u32,
    horizontal: u32,
    vertical: u32,
    palette: u32,
    important: u32,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    alpha_mask: u32,
    win: u32,
    unused2a: u128,
    unused2b: u128,
    unused2c: u32,
    red_gamma: u32,
    green_gamma: u32,
    blue_gamma: u32,
    intent: u32,
    profile_data: u32,
    profile_size: u32,
    reserved: u32,
}

///If MSB of the u128 is set, the other bits represent the digest. if MSB is 0, there is no digest
#[derive(Serialize, Deserialize, Clone)]
struct CompactOptionalDigest(u128);

impl Copy for CompactOptionalDigest {}

impl CompactOptionalDigest {
    fn new(optional_digest: Option<u128>) -> Self {
        let compact = match optional_digest {
            None => {0}
            Some(num) => {num | (1 << 127)}
        };
        Self(compact)
    }

    fn get(&self) -> Option<u128> {

        if self.0 & (1 << 127) != 0 {
            Some(self.0 & !(1u128 << 127))
        } else {
            None
        }
    }

    fn compare(&self, other: u128) -> bool {
        self.get().unwrap() == (other & !(1u128 << 127))
    }
}

#[derive(Serialize, Deserialize)]
struct B2BHeader {
    padding_size: u32,
    original_file_size: u32,
    signature: u128,
    od: CompactOptionalDigest,
}

#[derive(Serialize, Deserialize)]
pub struct Header {
    bmp: BitmapV5Header,
    b2b: B2BHeader,
}

impl BitmapV5Header {
    fn new(width: u32, height: u32, pixmap_size: u32) -> Self {
        let file_size = pixmap_size + BITMAP_HEADER_SIZE;

        Self {
            id: BITMAP_ID,
            file_size,
            unused1: 0,
            offset: BITMAP_HEADER_SIZE,
            dib_size: BITMAP_HEADER_SIZE - 14,
            width,
            height,
            pbnlanes: 1,
            bpp: BYTES_PER_PIXEL as u16 * 8,
            compression: 3,
            pixmap_size,
            horizontal: 4000,
            vertical: 4000,
            palette: 0,
            important: 0,
            red_mask: 0xFF0000,
            green_mask: 0xFF00,
            blue_mask: 0xFF,
            alpha_mask: 0xFF000000,
            win: 0x57696E20,
            unused2a: 0,
            unused2b: 0,
            unused2c: 0,
            red_gamma: 0,
            green_gamma: 0,
            blue_gamma: 0,
            intent: 0,
            profile_data: 0,
            profile_size: 0,
            reserved: 0
        }
    }
}

impl B2BHeader {
    fn new(padding_size: u32, file_size: u64, optional_digest: Option<u128>) -> Self {
        Self {
            padding_size,
            original_file_size: file_size as u32,
            signature: B2B_SIGNATURE,
            od: CompactOptionalDigest::new(optional_digest),
        }
    }
}

impl Header {
    pub fn new(file_size: u64, optional_digest: Option<u128>) -> Self {
        let (width, height, pixmap_size, padding_size) = Self::get_properties(file_size);

        Self {
            bmp: BitmapV5Header::new(width, height, pixmap_size),
            b2b: B2BHeader::new(padding_size, file_size, optional_digest),
        }
    }

    pub fn pixmap_size(&self) -> u32 {
        self.bmp.pixmap_size
    }

    pub fn padding_size(&self) -> u32 { self.b2b.padding_size }

    pub fn original_file_size(&self) -> u32 { self.b2b.original_file_size }

    /// If this check passes, then this means that there is a high chance that:
    /// a) the bitmap header is correct
    /// b) the b2b header is correct
    /// Point a) implies that the bitmap header has not been converted to a larger or smaller one at any point.
    /// Point b) implies that the bitmap was created by b2b.
    /// Of course there is a small chance that a V5 bitmap may contain the signature in that particular position
    pub fn check_signature(&self) -> Result<()> {
        if self.b2b.signature != B2B_SIGNATURE {
            Err(Error::new(ErrorKind::InvalidB2BSignature, ""))
        } else {
            Ok(())
        }
    }

    pub fn check_id(&self) -> Result<()> {
        if self.bmp.id != BITMAP_ID{
            Err(Error::new(ErrorKind::InvalidBitmapID, ""))
        } else {
            Ok(())
        }
    }

    pub fn check_padding_size(&self) -> Result<()> {
        if self.padding_size() >= self.pixmap_size() {
            Err(Error::new(ErrorKind::BadPaddingSize, ""))
        } else {
            Ok(())
        }
    }

    ///Returns a (verified, error) pair
    pub fn verify(&self, other_digest: u128) -> (bool, bool) {
        match self.b2b.od.get() {
            None => {
                //If the bitmap was created without the -v command, no digest was added. So verifying the created bitmap is not possible
                (false, true)
            }
            Some(_) => {
                (self.b2b.od.compare(other_digest), false)
            }
        }
    }
    /// Given the size of the file, calculate a suitable width and height for a pixmap (large enough to contain the file data but not so large as to
    /// have too much padding). Then calculate the padding required.
    fn get_properties(file_size: u64) -> (u32, u32, u32, u32) {

        let total_data_size = file_size as f32 + Self::b2b_header_size() as f32;

        let width = (total_data_size / Self::bytes_per_pixel() as f32).sqrt().ceil() as u32;

        let height = (total_data_size / (width as f32 * Self::bytes_per_pixel() as f32)).ceil() as u32;

        let pixmap_size = width * height * Self::bytes_per_pixel();

        let padding_size = pixmap_size - file_size as u32 - Self::b2b_header_size();

        (width, height, pixmap_size, padding_size)
    }

    pub const fn total_header_size() -> u32 { Self::bitmap_header_size() + Self::b2b_header_size() }

    pub const fn bitmap_header_size() -> u32 { BITMAP_HEADER_SIZE }

    pub const fn b2b_header_size() -> u32 { B2B_HEADER_SIZE }

    pub const fn bytes_per_pixel() -> u32 { 4 }
}
