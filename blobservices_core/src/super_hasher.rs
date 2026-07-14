use digest::Digest as _;

use crate::proto::core::BlobHashes;

pub struct SuperHasher {
    size: u64,
    crc32: u64,
    crc32c: u64,
    xxh64: xxhash_rust::xxh64::Xxh64,
    md5: md5::Md5,
    sha1: sha1::Sha1,
    sha256: sha2::Sha256,
    sha512: sha2::Sha512,
    sha3_256: sha3::Sha3_256,
    sha3_512: sha3::Sha3_512,
    blake2sp: blake2s_simd::blake2sp::State,
}

#[allow(clippy::new_without_default)]
impl SuperHasher {
    pub fn new() -> Self {
        Self {
            size: 0,
            crc32: crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32IsoHdlc, b""),
            crc32c: crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32Iscsi, b""),
            xxh64: xxhash_rust::xxh64::Xxh64::new(0),
            md5: md5::Md5::new(),
            sha1: sha1::Sha1::new(),
            sha256: sha2::Sha256::new(),
            sha512: sha2::Sha512::new(),
            sha3_256: sha3::Sha3_256::new(),
            sha3_512: sha3::Sha3_512::new(),
            blake2sp: blake2s_simd::blake2sp::State::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.size += data.len() as u64;
        self.crc32 = crc_fast::checksum_combine(
            crc_fast::CrcAlgorithm::Crc32IsoHdlc,
            self.crc32,
            crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32IsoHdlc, data),
            data.len() as u64,
        );
        self.crc32c = crc_fast::checksum_combine(
            crc_fast::CrcAlgorithm::Crc32Iscsi,
            self.crc32c,
            crc_fast::checksum(crc_fast::CrcAlgorithm::Crc32Iscsi, data),
            data.len() as u64,
        );
        self.xxh64.update(data);
        self.md5.update(data);
        self.sha1.update(data);
        self.sha256.update(data);
        self.sha512.update(data);
        self.sha3_256.update(data);
        self.sha3_512.update(data);
        self.blake2sp.update(data);
    }

    pub fn finalize(self) -> (u64, BlobHashes) {
        (
            self.size,
            BlobHashes {
                crc32: Some(self.crc32 as u32),
                crc32c: Some(self.crc32c as u32),
                xxh64: Some(self.xxh64.digest()),
                md5: Some(self.md5.finalize().to_vec()),
                sha1: Some(self.sha1.finalize().to_vec()),
                sha256: Some(self.sha256.finalize().to_vec()),
                sha256_dropbox: None, // TODO
                sha512: Some(self.sha512.finalize().to_vec()),
                sha3_256: Some(self.sha3_256.finalize().to_vec()),
                sha3_512: Some(self.sha3_512.finalize().to_vec()),
                blake2sp: Some(self.blake2sp.finalize().as_bytes().to_vec()),
            },
        )
    }
}
