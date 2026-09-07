use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use blobservices_core::{extractors::ResponseFormat, proto};
use sqlx::postgres::PgBindIterExt;

use crate::{NamespaceAndKey, state::AppState};

pub async fn get_blob_ref(
    State(state): State<AppState>,
    Path(nk): Path<NamespaceAndKey>,
    response_format: ResponseFormat,
) -> Result<Response, Response> {
    let res = sqlx::query!(
        r#"
        SELECT blobs.*
        FROM blob_references
        INNER JOIN blob_reference_revisions
            ON blob_reference_revisions.id = blob_references.current_revision_id
        INNER JOIN blobs ON blobs.id = blob_reference_revisions.blob_id
        WHERE namespace = $1 AND key = $2
        LIMIT 1
        "#,
        nk.namespace,
        nk.key
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND.into_response(),
        _ => {
            tracing::error!(err=?e, "FAILED_TO_QUERY_REF");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    })?;

    let blob_info = proto::manager::BlobInfo {
        id: res.id.as_bytes().to_vec(),
        size: res.size as u64,
        hashes: proto::core::BlobHashes {
            crc32: res.cs_crc32.map(|hash| hash as u32),
            crc32c: res.cs_crc32c.map(|hash| hash as u32),
            xxh64: res.cs_xxh64.map(|hash| hash as u64),
            md5: res.cs_md5,
            sha1: res.cs_sha1,
            sha256: res.cs_sha256,
            sha256_dropbox: res.cs_sha256_dropbox,
            sha512: res.cs_sha512,
            sha3_256: res.cs_sha3_256,
            sha3_512: res.cs_sha3_512,
            blake2sp: res.cs_blake2sp,
        },
    };
    let mut finder = BlobSourceFinder::new(&state.db_pool, (res.id, &blob_info));
    finder.resolve_blob().await?;

    let res = proto::manager::GetBlobRefResponse {
        locations: finder.find_locations().await?,
        related_blobs: finder.find_related_blobs().await?,
        operations: finder.operations,
        blob: blob_info,
    };

    Ok(response_format.message_to_response(res))
}

struct BlobSourceFinder<'a> {
    db_pool: &'a sqlx::Pool<sqlx::Postgres>,
    main_blob: (uuid::Uuid, &'a proto::manager::BlobInfo),

    known_blobs: HashSet<uuid::Uuid>,
    operations: Vec<proto::manager::BlobOperation>,
    after_first_resolve: bool,
}

impl<'a> BlobSourceFinder<'a> {
    pub fn new(
        db_pool: &'a sqlx::Pool<sqlx::Postgres>,
        main_blob: (uuid::Uuid, &'a proto::manager::BlobInfo),
    ) -> BlobSourceFinder<'a> {
        let mut known_blobs = HashSet::with_capacity(1);
        known_blobs.insert(main_blob.0);
        BlobSourceFinder {
            db_pool,
            main_blob,
            known_blobs,
            operations: Vec::new(),
            after_first_resolve: false,
        }
    }

    async fn find_locations(&self) -> Result<Vec<proto::manager::BlobLocation>, Response> {
        let res = sqlx::query!(
            "SELECT * FROM blob_locations WHERE blob_id = ANY($1) ORDER BY blob_id ASC",
            self.known_blobs.iter().bind_iter() as _
        )
        .fetch_all(self.db_pool)
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_QUERY_LOCATIONS");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?
        .into_iter()
        .map(|l| proto::manager::BlobLocation {
            blob_id: l.blob_id.into(),
            address: l.address,
            storage: l.storage_id,
        })
        .collect();
        Ok(res)
    }

    async fn find_related_blobs(&self) -> Result<Vec<proto::manager::BlobInfo>, Response> {
        let res = sqlx::query!(
            r#"
        SELECT blobs.*
        FROM blobs
        WHERE id = ANY($1) AND id != $2
        "#,
            self.known_blobs.iter().bind_iter() as _,
            self.main_blob.0
        )
        .fetch_all(self.db_pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND.into_response(),
            _ => {
                tracing::error!(err=?e, "FAILED_TO_QUERY_RELATED_BLOBS");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        })?
        .into_iter()
        .map(|res| proto::manager::BlobInfo {
            id: res.id.as_bytes().to_vec(),
            size: res.size as u64,
            hashes: proto::core::BlobHashes {
                crc32: res.cs_crc32.map(|hash| hash as u32),
                crc32c: res.cs_crc32c.map(|hash| hash as u32),
                xxh64: res.cs_xxh64.map(|hash| hash as u64),
                md5: res.cs_md5,
                sha1: res.cs_sha1,
                sha256: res.cs_sha256,
                sha256_dropbox: res.cs_sha256_dropbox,
                sha512: res.cs_sha512,
                sha3_256: res.cs_sha3_256,
                sha3_512: res.cs_sha3_512,
                blake2sp: res.cs_blake2sp,
            },
        })
        .collect();

        Ok(res)
    }

    async fn find_slices_superset(&mut self) -> Result<HashSet<uuid::Uuid>, Response> {
        let res = sqlx::query!(
            "
            SELECT
                src_blob_id, dst_blob_id, start
            FROM blob_slices
            WHERE dst_blob_id = ANY($1) AND src_blob_id != ALL($1)
            ",
            self.known_blobs.iter().bind_iter() as _
        )
        .fetch_all(self.db_pool)
        .await
        .map_err(|e| {
            tracing::error!(err=?e, "FAILED_TO_QUERY_SLICES");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?
        .into_iter()
        .map(|l| {
            (
                l.src_blob_id,
                proto::manager::BlobOperation {
                    src_blob_id: l.src_blob_id.into(),
                    dst_blob_id: l.dst_blob_id.into(),
                    operation: Some(proto::manager::blob_operation::Operation::Slice(
                        proto::manager::BlobSourceSlice {
                            src_start: Some(l.start as u64),
                            dst_start: None,
                            size: None,
                        },
                    )),
                },
            )
        });

        let mut new_ids = HashSet::new();
        for r in res {
            new_ids.insert(r.0);
            self.operations.push(r.1);
        }

        Ok(new_ids)
    }

    async fn resolve_blob(&mut self) -> Result<(), Response> {
        loop {
            let new_ids = self.find_slices_superset().await?;
            // TODO: find subsets, transformers

            if new_ids.is_empty() {
                break;
            }

            self.known_blobs.extend(new_ids);
            self.after_first_resolve = true;
        }

        Ok(())
    }
}
