ALTER TABLE blobs
ADD COLUMN cs_sha3_256 bytea CONSTRAINT blobs_cs_sha3_256_check CHECK (cs_sha3_256 IS NULL OR octet_length(cs_sha3_256) = 32);
