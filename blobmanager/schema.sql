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
    current_revision_id UUID NOT NULL,
    CONSTRAINT "UQ_br_namespace_key" UNIQUE (namespace, key)
);

CREATE TABLE blob_reference_revisions (
    id UUID PRIMARY KEY,
    reference_id UUID NOT NULL REFERENCES blob_references (id) ON DELETE NO ACTION,
    blob_id UUID NOT NULL REFERENCES blobs (id) ON DELETE NO ACTION,
    attributes JSONB NULL CHECK (attributes IS NULL OR jsonb_typeof(attributes) = 'object'),
    metadata JSONB NULL CHECK (metadata IS NULL OR jsonb_typeof(metadata) = 'object')
);
CREATE INDEX "IDX_brr_blob" ON blob_reference_revisions (blob_id);
CREATE UNIQUE INDEX "IDX_brr_refid_id" ON blob_reference_revisions (reference_id, id);

ALTER TABLE blob_references ADD FOREIGN KEY (id, current_revision_id) REFERENCES blob_reference_revisions (reference_id, id) ON DELETE NO ACTION DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE blob_locations (
    id UUID PRIMARY KEY,
    blob_id UUID NOT NULL REFERENCES blobs (id) ON DELETE NO ACTION,
    storage_id TEXT NOT NULL CHECK (storage_id != '' AND storage_id ~ '^[a-z0-9_-]+$'),
    address TEXT NOT NULL CHECK (address != ''),
    attributes JSONB NULL CHECK (attributes IS NULL OR jsonb_typeof(attributes) = 'object'),
    CONSTRAINT "UQ_bl_storage_address" UNIQUE (storage_id, address)
);
CREATE INDEX "IDX_bl_blob_storage" ON blob_locations (blob_id, storage_id);

CREATE TABLE blob_transform_shared_parameters (
    id UUID PRIMARY KEY,
    transform_type INTEGER NOT NULL,

    -- NULL の場合 shared parameter 用意する意味がないため
    parameters JSONB NOT NULL CHECK (jsonb_typeof(parameters) = 'object')
);
CREATE UNIQUE INDEX "IDX_btsp_tt_id" ON blob_transform_shared_parameters (transform_type, id);

CREATE TABLE blob_transform_shared_parameter_alias (
    id UUID PRIMARY KEY,

    namespace TEXT NOT NULL CHECK (namespace != ''),
    slug TEXT NOT NULL CHECK (slug != ''),
    parameter_id UUID NOT NULL REFERENCES blob_transform_shared_parameters(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX "IDX_btspa_ns_slug" ON blob_transform_shared_parameter_alias (namespace, slug);
CREATE INDEX "IDX_btspa_pid" ON blob_transform_shared_parameter_alias (parameter_id);

CREATE TABLE blob_transforms (
    id UUID PRIMARY KEY,
    src_blob_id UUID NOT NULL REFERENCES blobs(id) ON DELETE NO ACTION,
    dst_blob_id UUID NOT NULL REFERENCES blobs(id) ON DELETE NO ACTION,
    transform_type INTEGER NOT NULL,
    is_reversible BOOLEAN NOT NULL,

    -- shared_parameter がある場合、トップレベルだけ見て上書き
    -- (e.g. {"a": {"b": 1, "c": 2}, "d": 3} が shared, {"a": {"b": 2}} が固有だった場合、{"a": {"b": 2}, "d": 3} になる)
    shared_parameter_id UUID NULL,
    parameters JSONB NULL CHECK (parameters IS NULL OR jsonb_typeof(parameters) = 'object'),
    FOREIGN KEY (transform_type, shared_parameter_id) REFERENCES blob_transform_shared_parameters(transform_type, id) ON DELETE NO ACTION
);
CREATE UNIQUE INDEX "IDX_bt_src_dst" ON blob_transforms (src_blob_id, dst_blob_id);
CREATE INDEX "IDX_bt_dst_rev" ON blob_transforms (dst_blob_id, is_reversible DESC);
CREATE INDEX "IDX_bt_tt_spi" ON blob_transforms (transform_type, shared_parameter_id);

-- dst = src[start..<start+dst.size]
CREATE TABLE blob_slices (
    src_blob_id UUID NOT NULL REFERENCES blobs(id) ON DELETE NO ACTION,
    dst_blob_id UUID NOT NULL REFERENCES blobs(id) ON DELETE NO ACTION,
    start BIGINT NOT NULL CHECK (start >= 0),
    PRIMARY KEY (dst_blob_id, src_blob_id, start)
);

CREATE INDEX "IDX_bs_src_dst" ON blob_slices (src_blob_id, dst_blob_id);
