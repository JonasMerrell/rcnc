//! Modbus RTU framing, read requests only. rcnc never writes to the VFD: the only requests this module can
//! build are "read holding registers" (0x03, the H100's F parameters) and "read input registers" (0x04, its
//! live readings). There is no write function code here on purpose.

/// The two read requests there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// 0x03: the H100 answers its parameters here, register N = parameter FN (F143 = register 143).
    Holding,
    /// 0x04: the H100's live readings (register 0 = output frequency in 0.1 Hz, 1 = set frequency, ...).
    Input,
}

impl Read {
    pub fn code(self) -> u8 {
        match self {
            Read::Holding => 0x03,
            Read::Input => 0x04,
        }
    }
}

/// Modbus CRC-16 (polynomial 0xA001, start 0xFFFF), sent low byte first.
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for b in bytes {
        crc ^= *b as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xA001 } else { crc >> 1 };
        }
    }
    crc
}

/// The 8-byte request for `count` registers from `start` at slave `addr`.
pub fn request(addr: u8, read: Read, start: u16, count: u16) -> [u8; 8] {
    let mut f = [addr, read.code(), (start >> 8) as u8, start as u8, (count >> 8) as u8, count as u8, 0, 0];
    let crc = crc16(&f[..6]);
    f[6] = crc as u8;
    f[7] = (crc >> 8) as u8;
    f
}

/// How many bytes the answer to `count` registers is (address, function, byte count, data, CRC).
pub fn reply_len(count: u16) -> usize {
    5 + 2 * count as usize
}

/// An exception answer is 5 bytes (address, function | 0x80, code, CRC).
pub const EXCEPTION_LEN: usize = 5;

/// Decode an answer: the registers, or why not.
pub fn parse(addr: u8, read: Read, count: u16, bytes: &[u8]) -> Result<Vec<u16>, String> {
    if bytes.len() >= EXCEPTION_LEN && bytes[0] == addr && bytes[1] == read.code() | 0x80 {
        let ok = crc16(&bytes[..3]) == u16::from_le_bytes([bytes[3], bytes[4]]);
        return Err(if ok { format!("the VFD refused the read (Modbus exception {})", bytes[2]) } else { "garbled answer (bad CRC)".into() });
    }
    let n = reply_len(count);
    if bytes.len() < n {
        return Err(if bytes.is_empty() { "no answer".into() } else { format!("short answer ({} of {n} bytes)", bytes.len()) });
    }
    let b = &bytes[..n];
    if crc16(&b[..n - 2]) != u16::from_le_bytes([b[n - 2], b[n - 1]]) {
        return Err("garbled answer (bad CRC)".into());
    }
    if b[0] != addr || b[1] != read.code() || b[2] as usize != 2 * count as usize {
        return Err(format!("unexpected answer {:02X?}", b));
    }
    Ok(b[3..n - 2].chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_match_the_known_h100_frames() {
        // FluidNC's tested H100 notes: "[01] [04] [0000] [0002] -- output frequency"; the standard example
        // CRC for 01 03 00 00 00 01 is 84 0A.
        assert_eq!(request(1, Read::Holding, 0, 1), [0x01, 0x03, 0x00, 0x00, 0x00, 0x01, 0x84, 0x0A]);
        let r = request(1, Read::Input, 0, 2);
        assert_eq!(&r[..6], &[0x01, 0x04, 0x00, 0x00, 0x00, 0x02]);
        assert_eq!(crc16(&r), 0, "a frame with its CRC checks to zero");
        // F143 (poles), as grblHAL reads it
        assert_eq!(&request(1, Read::Holding, 143, 1)[..6], &[0x01, 0x03, 0x00, 0x8F, 0x00, 0x01]);
    }

    #[test]
    fn answers_decode_and_bad_ones_are_refused() {
        let mut a = vec![0x01, 0x04, 0x04, 0x05, 0xDC, 0x05, 0xDC];
        let c = crc16(&a);
        a.extend_from_slice(&c.to_le_bytes());
        assert_eq!(parse(1, Read::Input, 2, &a).unwrap(), vec![1500, 1500]);
        let mut bad = a.clone();
        bad[4] ^= 1;
        assert!(parse(1, Read::Input, 2, &bad).unwrap_err().contains("CRC"));
        assert!(parse(2, Read::Input, 2, &a).is_err(), "another slave's answer");
        assert!(parse(1, Read::Input, 2, &a[..5]).unwrap_err().contains("short"));
        assert_eq!(parse(1, Read::Input, 2, &[]).unwrap_err(), "no answer");
        let mut ex = vec![0x01, 0x84, 0x02];
        let c = crc16(&ex);
        ex.extend_from_slice(&c.to_le_bytes());
        assert!(parse(1, Read::Input, 2, &ex).unwrap_err().contains("exception 2"));
    }
}
