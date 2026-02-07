use core::fmt;

/**
    A lightweight cursor-based reader for binary data.

    Tracks position internally, providing bounds-checked reads for
    common integer types and byte slices. Used by format crates
    that parse binary TLV structures (BCert, XMR, etc.).
*/
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

/**
    Error returned when a [`Reader`] operation fails.
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    pub needed: usize,
    pub have: usize,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unexpected end of data: need {} bytes, have {}",
            self.needed, self.have
        )
    }
}

impl std::error::Error for ReadError {}

impl<'a> Reader<'a> {
    /**
        Create a new reader over the given byte slice.
    */
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /**
        Current byte offset within the underlying data.
    */
    pub const fn position(&self) -> usize {
        self.pos
    }

    /**
        Number of bytes remaining from the current position.
    */
    pub const fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /**
        The full underlying byte slice.
    */
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /**
        Check that at least `n` bytes remain.
    */
    pub const fn ensure(&self, n: usize) -> Result<(), ReadError> {
        if self.remaining() < n {
            Err(ReadError {
                needed: self.pos + n,
                have: self.data.len(),
            })
        } else {
            Ok(())
        }
    }

    /**
        Read exactly `n` bytes, advancing the position.
    */
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ReadError> {
        self.ensure(n)?;
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /**
        Read a fixed-size byte array, advancing the position.
    */
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ReadError> {
        let b = self.read_bytes(N)?;
        let mut arr = [0u8; N];
        arr.copy_from_slice(b);
        Ok(arr)
    }

    /**
        Read a big-endian `u16`.
    */
    pub fn read_u16be(&mut self) -> Result<u16, ReadError> {
        Ok(u16::from_be_bytes(self.read_array()?))
    }

    /**
        Read a big-endian `u32`.
    */
    pub fn read_u32be(&mut self) -> Result<u32, ReadError> {
        Ok(u32::from_be_bytes(self.read_array()?))
    }

    /**
        Read a little-endian `u16`.
    */
    pub fn read_u16le(&mut self) -> Result<u16, ReadError> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    /**
        Read a little-endian `u32`.
    */
    pub fn read_u32le(&mut self) -> Result<u32, ReadError> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    /**
        Read a null-terminated, 4-byte-aligned string field.

        `raw_len` is the declared byte length before alignment padding.
        The reader advances past `raw_len` rounded up to the next
        multiple of 4.
    */
    pub fn read_padded_string(&mut self, raw_len: usize) -> Result<String, ReadError> {
        let aligned = (raw_len + 3) & !3;
        let bytes = self.read_bytes(aligned)?;
        let end = bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(raw_len.min(aligned));
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_integers() {
        let data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let mut r = Reader::new(&data);
        assert_eq!(r.read_u16be().unwrap(), 0x0001);
        assert_eq!(r.read_u32be().unwrap(), 0x02030405);
        assert_eq!(r.remaining(), 2);
    }

    #[test]
    fn read_le_integers() {
        let data = [0x01, 0x00, 0x04, 0x03, 0x02, 0x01];
        let mut r = Reader::new(&data);
        assert_eq!(r.read_u16le().unwrap(), 0x0001);
        assert_eq!(r.read_u32le().unwrap(), 0x01020304);
    }

    #[test]
    fn read_array_fixed() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD];
        let mut r = Reader::new(&data);
        let arr: [u8; 4] = r.read_array().unwrap();
        assert_eq!(arr, [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn read_past_end() {
        let data = [0x00, 0x01];
        let mut r = Reader::new(&data);
        assert!(r.read_u32be().is_err());
    }

    #[test]
    fn position_tracking() {
        let data = [0; 16];
        let mut r = Reader::new(&data);
        assert_eq!(r.position(), 0);
        r.read_bytes(5).unwrap();
        assert_eq!(r.position(), 5);
        assert_eq!(r.remaining(), 11);
    }

    #[test]
    fn read_padded_string_aligned() {
        // "abc" + null + padding to align to 4
        let data = [b'a', b'b', b'c', 0x00];
        let mut r = Reader::new(&data);
        let s = r.read_padded_string(4).unwrap();
        assert_eq!(s, "abc");
    }

    #[test]
    fn read_padded_string_needs_padding() {
        // raw_len=3, aligned=4 -> reads 4 bytes
        let data = [b'h', b'i', 0x00, 0x00, 0xFF];
        let mut r = Reader::new(&data);
        let s = r.read_padded_string(3).unwrap();
        assert_eq!(s, "hi");
        assert_eq!(r.position(), 4);
    }
}
