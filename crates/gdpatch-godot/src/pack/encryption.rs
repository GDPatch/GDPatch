//! Godot encrypted file access. See core/io/file_access_encrypted.cpp.

use crate::Error;
use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use byteorder::{LittleEndian, ReadBytesExt};
use md5::{Digest, Md5};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::{Read, Seek, SeekFrom};
use std::str::FromStr;

/// A 256-bit Godot encryption key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionKey([u8; 32]);

impl FromStr for EncryptionKey {
    type Err = Error;

    fn from_str(value: &str) -> crate::Result<Self> {
        if value.len() != 64 || !value.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::InvalidEncryptionKey);
        }
        let mut bytes = [0; 32];
        for (out, pair) in bytes.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
            let digit = |c: u8| match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => unreachable!(),
            };
            *out = digit(pair[0]) * 16 + digit(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl<'de> Deserialize<'de> for EncryptionKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl Serialize for EncryptionKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value: String = self.0.iter().map(|byte| format!("{byte:02x}")).collect();
        serializer.serialize_str(&value)
    }
}

/// Read and decrypt a packed file or directory.
pub(super) fn decrypt<R: Read + Seek>(
    reader: &mut R,
    key: &EncryptionKey,
    expected_size: Option<u64>,
) -> crate::Result<Vec<u8>> {
    let mut hash = [0; 16];
    reader.read_exact(&mut hash)?;
    let length = reader.read_u64::<LittleEndian>()?;
    if expected_size.is_some_and(|size| size != length) {
        return Err(Error::BadData);
    }
    let padded = length.checked_add(15).ok_or(Error::BadData)? & !15;
    let mut iv = [0; 16];
    reader.read_exact(&mut iv)?;

    let start = reader.stream_position()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    if padded > end.checked_sub(start).ok_or(Error::BadData)? {
        return Err(Error::TooShort);
    }
    let padded = usize::try_from(padded).map_err(|_| Error::BadData)?;
    let mut data = Vec::new();
    data.try_reserve_exact(padded).map_err(|_| Error::BadData)?;
    data.resize(padded, 0);
    reader.read_exact(&mut data)?;
    cfb_mode::Decryptor::<aes::Aes256>::new((&key.0).into(), (&iv).into()).decrypt(&mut data);
    data.truncate(length as usize);
    if Md5::digest(&data).as_slice() != hash {
        return Err(Error::DecryptionFailed);
    }
    Ok(data)
}
