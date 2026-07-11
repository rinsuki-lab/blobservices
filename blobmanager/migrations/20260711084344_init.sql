CREATE TABLE IF NOT EXISTS blobs (
    id uuid,
    size bigint NOT NULL,
    cs_crc32 integer,
    cs_crc32c integer,
    cs_xxh64 bigint,
    cs_md5 bytea,
    cs_sha1 bytea,
    cs_sha256 bytea,
    cs_sha256_dropbox bytea,
    cs_sha512 bytea,
    cs_sha3_512 bytea,
    cs_blake2sp bytea,
    CONSTRAINT blobs_pkey PRIMARY KEY (id),
    CONSTRAINT blobs_cs_blake2sp_check CHECK (cs_blake2sp IS NULL OR octet_length(cs_blake2sp) = 32),
    CONSTRAINT blobs_cs_md5_check CHECK (cs_md5 IS NULL OR octet_length(cs_md5) = 16),
    CONSTRAINT blobs_cs_sha1_check CHECK (cs_sha1 IS NULL OR octet_length(cs_sha1) = 20),
    CONSTRAINT blobs_cs_sha256_check CHECK (cs_sha256 IS NULL OR octet_length(cs_sha256) = 32),
    CONSTRAINT blobs_cs_sha256_dropbox_check CHECK (cs_sha256_dropbox IS NULL OR octet_length(cs_sha256_dropbox) = 32),
    CONSTRAINT blobs_cs_sha3_512_check CHECK (cs_sha3_512 IS NULL OR octet_length(cs_sha3_512) = 64),
    CONSTRAINT blobs_cs_sha512_check CHECK (cs_sha512 IS NULL OR octet_length(cs_sha512) = 64),
    CONSTRAINT blobs_size_check CHECK (size >= 0)
);

COMMENT ON COLUMN blobs.cs_crc32 IS 'IEEE 802.3準拠。u32をbitcastでi32に変換して保存する';

COMMENT ON COLUMN blobs.cs_crc32c IS 'u32をbitcastでi32に変換して保存する';

COMMENT ON COLUMN blobs.cs_xxh64 IS 'seedは0。u64をbitcastでi64に変換して保存する';

COMMENT ON COLUMN blobs.cs_sha256_dropbox IS 'ref: https://www.dropbox.com/developers/reference/content-hash';

CREATE INDEX IF NOT EXISTS "IDX_blob_size" ON blobs (size);

CREATE TABLE IF NOT EXISTS blob_locations (
    id uuid,
    blob_id uuid NOT NULL,
    storage_id text NOT NULL,
    address text NOT NULL,
    attributes jsonb,
    CONSTRAINT blob_locations_pkey PRIMARY KEY (id),
    CONSTRAINT "UQ_bl_storage_address" UNIQUE (storage_id, address),
    CONSTRAINT blob_locations_blob_id_fkey FOREIGN KEY (blob_id) REFERENCES blobs (id),
    CONSTRAINT blob_locations_address_check CHECK (address <> ''::text),
    CONSTRAINT blob_locations_attributes_check CHECK (attributes IS NULL OR jsonb_typeof(attributes) = 'object'::text),
    CONSTRAINT blob_locations_storage_id_check CHECK (storage_id <> ''::text AND storage_id ~ '^[a-z0-9_-]+$'::text)
);

CREATE INDEX IF NOT EXISTS "IDX_bl_blob_storage" ON blob_locations (blob_id, storage_id);

CREATE TABLE IF NOT EXISTS blob_references (
    id uuid,
    namespace text NOT NULL,
    key text NOT NULL,
    blob_id uuid NOT NULL,
    attributes jsonb,
    metadata jsonb,
    CONSTRAINT blob_references_pkey PRIMARY KEY (id),
    CONSTRAINT "UQ_br_namespace_key" UNIQUE (namespace, key),
    CONSTRAINT blob_references_blob_id_fkey FOREIGN KEY (blob_id) REFERENCES blobs (id),
    CONSTRAINT blob_references_attributes_check CHECK (attributes IS NULL OR jsonb_typeof(attributes) = 'object'::text),
    CONSTRAINT blob_references_key_check CHECK (length(key) >= 1 AND length(key) <= 2047),
    CONSTRAINT blob_references_metadata_check CHECK (metadata IS NULL OR jsonb_typeof(metadata) = 'object'::text),
    CONSTRAINT blob_references_namespace_check CHECK (length(namespace) >= 1 AND length(namespace) <= 127 AND namespace ~ '^[a-z0-9._-]+$'::text)
);

CREATE INDEX IF NOT EXISTS "IDX_br_blob" ON blob_references (blob_id);
