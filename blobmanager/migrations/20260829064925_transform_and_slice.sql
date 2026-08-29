CREATE TABLE IF NOT EXISTS blob_slices (
    src_blob_id uuid,
    dst_blob_id uuid,
    start bigint,
    CONSTRAINT blob_slices_pkey PRIMARY KEY (dst_blob_id, src_blob_id, start),
    CONSTRAINT blob_slices_start_check CHECK (start >= 0)
);

CREATE INDEX IF NOT EXISTS "IDX_bs_src_dst" ON blob_slices (src_blob_id, dst_blob_id);

CREATE TABLE IF NOT EXISTS blob_transform_shared_parameters (
    id uuid,
    transform_type integer NOT NULL,
    parameters jsonb NOT NULL,
    CONSTRAINT blob_transform_shared_parameters_pkey PRIMARY KEY (id),
    CONSTRAINT blob_transform_shared_parameters_parameters_check CHECK (jsonb_typeof(parameters) = 'object'::text)
);

CREATE UNIQUE INDEX IF NOT EXISTS "IDX_btsp_tt_id" ON blob_transform_shared_parameters (transform_type, id);

CREATE TABLE IF NOT EXISTS blob_transform_shared_parameter_alias (
    id uuid,
    namespace text NOT NULL,
    slug text NOT NULL,
    parameter_id uuid NOT NULL,
    CONSTRAINT blob_transform_shared_parameter_alias_pkey PRIMARY KEY (id),
    CONSTRAINT blob_transform_shared_parameter_alias_parameter_id_fkey FOREIGN KEY (parameter_id) REFERENCES blob_transform_shared_parameters (id) ON DELETE CASCADE,
    CONSTRAINT blob_transform_shared_parameter_alias_namespace_check CHECK (namespace <> ''::text),
    CONSTRAINT blob_transform_shared_parameter_alias_slug_check CHECK (slug <> ''::text)
);

CREATE UNIQUE INDEX IF NOT EXISTS "IDX_btspa_ns_slug" ON blob_transform_shared_parameter_alias (namespace, slug);

CREATE INDEX IF NOT EXISTS "IDX_btspa_pid" ON blob_transform_shared_parameter_alias (parameter_id);

CREATE TABLE IF NOT EXISTS blob_transforms (
    id uuid,
    src_blob_id uuid NOT NULL,
    dst_blob_id uuid NOT NULL,
    transform_type integer NOT NULL,
    is_reversible boolean NOT NULL,
    shared_parameter_id uuid,
    parameters jsonb,
    CONSTRAINT blob_transforms_pkey PRIMARY KEY (id),
    CONSTRAINT blob_transforms_transform_type_shared_parameter_id_fkey FOREIGN KEY (transform_type, shared_parameter_id) REFERENCES blob_transform_shared_parameters (transform_type, id),
    CONSTRAINT blob_transforms_parameters_check CHECK (parameters IS NULL OR jsonb_typeof(parameters) = 'object'::text)
);

CREATE INDEX IF NOT EXISTS "IDX_bt_dst_rev" ON blob_transforms (dst_blob_id, is_reversible DESC);

CREATE UNIQUE INDEX IF NOT EXISTS "IDX_bt_src_dst" ON blob_transforms (src_blob_id, dst_blob_id);

CREATE INDEX IF NOT EXISTS "IDX_bt_tt_spi" ON blob_transforms (transform_type, shared_parameter_id);

ALTER TABLE blob_slices
ADD CONSTRAINT blob_slices_dst_blob_id_fkey FOREIGN KEY (dst_blob_id) REFERENCES blobs (id);

ALTER TABLE blob_slices
ADD CONSTRAINT blob_slices_src_blob_id_fkey FOREIGN KEY (src_blob_id) REFERENCES blobs (id);

ALTER TABLE blob_transforms
ADD CONSTRAINT blob_transforms_dst_blob_id_fkey FOREIGN KEY (dst_blob_id) REFERENCES blobs (id);

ALTER TABLE blob_transforms
ADD CONSTRAINT blob_transforms_src_blob_id_fkey FOREIGN KEY (src_blob_id) REFERENCES blobs (id);
