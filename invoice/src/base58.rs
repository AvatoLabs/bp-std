// Written in 2014 by Andrew Poelstra <apoelstra@wpsoftware.net>
// SPDX-License-Identifier: CC0-1.0

//! Base58 encoder and decoder.
//!
//! This module provides functions for encoding and decoding base58 slices and
//! strings respectively.

use core::{fmt, iter, slice, str};

use commit_verify::{Digest, Sha256};

static BASE58_CHARS: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[rustfmt::skip]
static BASE58_DIGITS: [Option<u8>; 128] = [
    None,     None,     None,     None,     None,     None,     None,     None,     // 0-7
    None,     None,     None,     None,     None,     None,     None,     None,     // 8-15
    None,     None,     None,     None,     None,     None,     None,     None,     // 16-23
    None,     None,     None,     None,     None,     None,     None,     None,     // 24-31
    None,     None,     None,     None,     None,     None,     None,     None,     // 32-39
    None,     None,     None,     None,     None,     None,     None,     None,     // 40-47
    None,     Some(0),  Some(1),  Some(2),  Some(3),  Some(4),  Some(5),  Some(6),  // 48-55
    Some(7),  Some(8),  None,     None,     None,     None,     None,     None,     // 56-63
    None,     Some(9),  Some(10), Some(11), Some(12), Some(13), Some(14), Some(15), // 64-71
    Some(16), None,     Some(17), Some(18), Some(19), Some(20), Some(21), None,     // 72-79
    Some(22), Some(23), Some(24), Some(25), Some(26), Some(27), Some(28), Some(29), // 80-87
    Some(30), Some(31), Some(32), None,     None,     None,     None,     None,     // 88-95
    None,     Some(33), Some(34), Some(35), Some(36), Some(37), Some(38), Some(39), // 96-103
    Some(40), Some(41), Some(42), Some(43), None,     Some(44), Some(45), Some(46), // 104-111
    Some(47), Some(48), Some(49), Some(50), Some(51), Some(52), Some(53), Some(54), // 112-119
    Some(55), Some(56), Some(57), None,     None,     None,     None,     None,     // 120-127
];

/// Decodes a base58-encoded string into a byte vector.
pub fn decode(data: &str) -> Result<Vec<u8>, Error> {
    // 11/15 is just over log_256(58)
    let mut scratch = vec![0u8; 1 + data.len() * 11 / 15];
    // Build in base 256
    for d58 in data.bytes() {
        // Compute "X = X * 58 + next_digit" in base 256
        if d58 as usize >= BASE58_DIGITS.len() {
            return Err(Error::BadByte(d58));
        }
        let mut carry = match BASE58_DIGITS[d58 as usize] {
            Some(d58) => d58 as u32,
            None => {
                return Err(Error::BadByte(d58));
            }
        };
        for d256 in scratch.iter_mut().rev() {
            carry += *d256 as u32 * 58;
            *d256 = carry as u8;
            carry /= 256;
        }
        assert_eq!(carry, 0);
    }

    // Copy leading zeroes directly
    let mut ret: Vec<u8> = data.bytes().take_while(|&x| x == BASE58_CHARS[0]).map(|_| 0).collect();
    // Copy rest of string
    ret.extend(scratch.into_iter().skip_while(|&x| x == 0));
    Ok(ret)
}

/// Decodes a base58check-encoded string into a byte vector verifying the checksum.
pub fn decode_check(data: &str) -> Result<Vec<u8>, Error> {
    let mut ret: Vec<u8> = decode(data)?;
    if ret.len() < 4 {
        return Err(Error::TooShort(ret.len()));
    }
    let check_start = ret.len() - 4;

    let hash_check = Sha256::digest(&ret[..check_start]);
    let hash_check = Sha256::digest(hash_check)[..4].try_into().expect("4 byte slice");
    let data_check = ret[check_start..].try_into().expect("4 byte slice");

    let expected = u32::from_le_bytes(hash_check);
    let actual = u32::from_le_bytes(data_check);

    if expected != actual {
        return Err(Error::BadChecksum(expected, actual));
    }

    ret.truncate(check_start);
    Ok(ret)
}

/// Encodes `data` as a base58 string (see also `base58::encode_check()`).
pub fn encode(data: &[u8]) -> String { encode_iter(data.iter().cloned()) }

/// Encodes `data` as a base58 string including the checksum.
///
/// The checksum is the first four bytes of the sha256d of the data, concatenated onto the end.
pub fn encode_check(data: &[u8]) -> String {
    let checksum = Sha256::digest(data);
    let checksum = Sha256::digest(checksum);
    encode_iter(data.iter().cloned().chain(checksum[0..4].iter().cloned()))
}

/// Encodes a slice as base58, including the checksum, into a formatter.
///
/// The checksum is the first four bytes of the sha256d of the data, concatenated onto the end.
pub fn encode_check_to_fmt(fmt: &mut fmt::Formatter, data: &[u8]) -> fmt::Result {
    let checksum = Sha256::digest(data);
    let checksum = Sha256::digest(checksum);
    let iter = data.iter().cloned().chain(checksum[0..4].iter().cloned());
    format_iter(fmt, iter)
}

fn encode_iter<I>(data: I) -> String
where I: Iterator<Item = u8> + Clone {
    let mut ret = String::new();
    format_iter(&mut ret, data).expect("writing into string shouldn't fail");
    ret
}

fn format_iter<I, W>(writer: &mut W, data: I) -> Result<(), fmt::Error>
where
    I: Iterator<Item = u8> + Clone,
    W: fmt::Write,
{
    let mut ret = SmallVec::new();

    let mut leading_zero_count = 0;
    let mut leading_zeroes = true;
    // Build string in little endian with 0-58 in place of characters...
    for d256 in data {
        let mut carry = d256 as usize;
        if leading_zeroes && carry == 0 {
            leading_zero_count += 1;
        } else {
            leading_zeroes = false;
        }

        for ch in ret.iter_mut() {
            let new_ch = *ch as usize * 256 + carry;
            *ch = (new_ch % 58) as u8;
            carry = new_ch / 58;
        }
        while carry > 0 {
            ret.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    // ... then reverse it and convert to chars
    for _ in 0..leading_zero_count {
        ret.push(0);
    }

    for ch in ret.iter().rev() {
        writer.write_char(BASE58_CHARS[*ch as usize] as char)?;
    }

    Ok(())
}

/// Vector-like object that holds the first 100 elements on the stack. If more space is needed it
/// will be allocated on the heap.
struct SmallVec<T> {
    len: usize,
    stack: [T; 100],
    heap: Vec<T>,
}

impl<T: Default + Copy> SmallVec<T> {
    fn new() -> SmallVec<T> {
        SmallVec {
            len: 0,
            stack: [T::default(); 100],
            heap: Vec::new(),
        }
    }

    fn push(&mut self, val: T) {
        if self.len < 100 {
            self.stack[self.len] = val;
            self.len += 1;
        } else {
            self.heap.push(val);
        }
    }

    fn iter(&self) -> iter::Chain<slice::Iter<'_, T>, slice::Iter<'_, T>> {
        // If len<100 then we just append an empty vec
        self.stack[0..self.len].iter().chain(self.heap.iter())
    }

    fn iter_mut(&mut self) -> iter::Chain<slice::IterMut<'_, T>, slice::IterMut<'_, T>> {
        // If len<100 then we just append an empty vec
        self.stack[0..self.len].iter_mut().chain(self.heap.iter_mut())
    }
}

/// An error that might occur during base58 decoding.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Error {
    /// Invalid character encountered.
    BadByte(u8),
    /// Checksum was not correct (expected, actual).
    BadChecksum(u32, u32),
    /// The length (in bytes) of the object was not correct.
    ///
    /// Note that if the length is excessively long the provided length may be an estimate (and the
    /// checksum step may be skipped).
    InvalidLength(usize),
    /// Extended Key version byte(s) were not recognized.
    InvalidExtendedKeyVersion([u8; 4]),
    /// Address version byte were not recognized.
    InvalidAddressVersion(u8),
    /// Checked data was less than 4 bytes.
    TooShort(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::BadByte(b) => write!(f, "invalid base58 character {b:#x}"),
            Error::BadChecksum(exp, actual) => {
                write!(f, "base58ck checksum {actual:#x} does not match expected {exp:#x}")
            }
            Error::InvalidLength(ell) => write!(f, "length {ell} invalid for this base58 type"),
            Error::InvalidExtendedKeyVersion(ref v) => {
                write!(f, "extended key version {v:#04x?} is invalid for this base58 type")
            }
            Error::InvalidAddressVersion(ref v) => {
                write!(f, "address version {v} is invalid for this base58 type")
            }
            Error::TooShort(_) => write!(f, "base58ck data not even long enough for a checksum"),
        }
    }
}
