// Copyright 2019 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error;
use sha2::{Digest, Sha256};
use std::io::{self, Read};

pub(crate) struct DigestAdapter<T, D> {
    reader: T,
    hash: Vec<u8>,
    digest: Option<D>,
}

impl<T: Read> DigestAdapter<T, Sha256> {
    pub(crate) fn sha256(reader: T, hash: &[u8]) -> Self {
        Self {
            reader,
            hash: hash.to_owned(),
            digest: Some(Sha256::new()),
        }
    }
}

impl<T: Read, D: Digest> Read for DigestAdapter<T, D> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        assert!(
            self.digest.is_some(),
            "DigestAdapter::read called after end of file"
        );

        let size = self.reader.read(buf)?;
        if size == 0 {
            let result = std::mem::replace(&mut self.digest, None).unwrap().result();
            if result.as_slice() != self.hash.as_slice() {
                error::HashMismatch {
                    calculated: hex::encode(result),
                    expected: hex::encode(&self.hash),
                }
                .fail()?;
            }
            Ok(size)
        } else if let Some(digest) = &mut self.digest {
            digest.input(&buf[..size]);
            Ok(size)
        } else {
            unreachable!();
        }
    }
}

pub(crate) struct MaxSizeAdapter<T> {
    reader: T,
    size: usize,
    counter: usize,
}

impl<T> MaxSizeAdapter<T> {
    pub(crate) fn new(reader: T, size: usize) -> Self {
        Self {
            reader,
            size,
            counter: 0,
        }
    }
}

impl<T: Read> Read for MaxSizeAdapter<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = self.reader.read(buf)?;
        self.counter += size;
        if self.counter > self.size {
            error::MaxSizeExceeded { size: self.size }.fail()?;
        }
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use crate::io::{DigestAdapter, MaxSizeAdapter};
    use hex_literal::hex;
    use std::io::{Cursor, Read};

    #[test]
    fn test_max_size_adapter() {
        let mut reader = MaxSizeAdapter::new(Cursor::new(b"hello".to_vec()), 5);
        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_ok());
        assert_eq!(buf, b"hello");

        let mut reader = MaxSizeAdapter::new(Cursor::new(b"hello".to_vec()), 4);
        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_err());
    }

    #[test]
    fn test_digest_adapter() {
        let mut reader = DigestAdapter::sha256(
            Cursor::new(b"hello".to_vec()),
            &hex!("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"),
        );
        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_ok());
        assert_eq!(buf, b"hello");

        let mut reader = DigestAdapter::sha256(
            Cursor::new(b"hello".to_vec()),
            &hex!("0ebdc3317b75839f643387d783535adc360ca01f33c75f7c1e7373adcd675c0b"),
        );
        let mut buf = Vec::new();
        assert!(reader.read_to_end(&mut buf).is_err());
    }
}