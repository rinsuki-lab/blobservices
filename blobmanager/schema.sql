CREATE TABLE blobs (
    id UUID PRIMARY KEY,
    size BIGINT NOT NULL CHECK (size >= 0),
    cs_crc32 INTEGER NULL,
    cs_crc32c INTEGER NULL,
    cs_xxh64 BIGINT NULL,
    cs_md5 BYTEA NULL CHECK (cs_md5 IS NULL OR octet_length(cs_md5) = 16),
    cs_sha1 BYTEA NULL CHECK (cs_sha1 IS NULL OR octet_length(cs_sha1) = 20),
    cs_sha256 BYTEA NULL CHECK (cs_sha256 IS NULL OR octet_length(cs_sha256) = 32),
    cs_sha256_dropbox BYTEA NULL CHECK (cs_sha256_dropbox IS NULL OR octet_length(cs_sha256_dropbox) = 32),
    cs_sha512 BYTEA NULL CHECK (cs_sha512 IS NULL OR octet_length(cs_sha512) = 64),
    cs_sha3_256 BYTEA NULL CHECK (cs_sha3_256 IS NULL OR octet_length(cs_sha3_256) = 32),
    cs_sha3_512 BYTEA NULL CHECK (cs_sha3_512 IS NULL OR octet_length(cs_sha3_512) = 64),
    cs_blake2sp BYTEA NULL CHECK (cs_blake2sp IS NULL OR octet_length(cs_blake2sp) = 32)
);
CREATE INDEX "IDX_blob_size" ON blobs (size);
COMMENT ON COLUMN blobs.cs_crc32 IS 'IEEE 802.3準拠。u32をbitcastでi32に変換して保存する';
COMMENT ON COLUMN blobs.cs_crc32c IS 'u32をbitcastでi32に変換して保存する';
COMMENT ON COLUMN blobs.cs_xxh64 IS 'seedは0。u64をbitcastでi64に変換して保存する';
COMMENT ON COLUMN blobs.cs_sha256_dropbox IS 'ref: https://www.dropbox.com/developers/reference/content-hash';

CREATE TABLE blob_references (
    id UUID PRIMARY KEY,
    namespace TEXT NOT NULL CHECK (length(namespace) BETWEEN 1 AND 127 AND namespace ~ '^[a-z0-9._-]+$'),
    key TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 2047),
    blob_id UUID NOT NULL REFERENCES blobs (id) ON DELETE NO ACTION,
    attributes JSONB NULL CHECK (attributes IS NULL OR jsonb_typeof(attributes) = 'object'),
    metadata JSONB NULL CHECK (metadata IS NULL OR jsonb_typeof(metadata) = 'object'),
    CONSTRAINT "UQ_br_namespace_key" UNIQUE (namespace, key)
);
CREATE INDEX "IDX_br_blob" ON blob_references (blob_id);

CREATE TABLE blob_locations (
    id UUID PRIMARY KEY,
    blob_id UUID NOT NULL REFERENCES blobs (id) ON DELETE NO ACTION,
    storage_id TEXT NOT NULL CHECK (storage_id != '' AND storage_id ~ '^[a-z0-9_-]+$'),
    address TEXT NOT NULL CHECK (address != ''),
    attributes JSONB NULL CHECK (attributes IS NULL OR jsonb_typeof(attributes) = 'object'),
    CONSTRAINT "UQ_bl_storage_address" UNIQUE (storage_id, address)
);
CREATE INDEX "IDX_bl_blob_storage" ON blob_locations (blob_id, storage_id);
