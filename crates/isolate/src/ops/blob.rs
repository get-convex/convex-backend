use anyhow::Context;
use deno_core::{
    JsBuffer,
    ToJsBuffer,
};
use uuid::Uuid;

use super::OpProvider;

#[convex_macro::v8_op]
pub fn op_blob_create_part<'b, P: OpProvider<'b>>(
    provider: &mut P,
    bytes: JsBuffer,
) -> anyhow::Result<Uuid> {
    provider.create_blob_part(bytes.into())
}

#[convex_macro::v8_op]
pub fn op_blob_slice_part<'b, P: OpProvider<'b>>(
    provider: &mut P,
    id: Uuid,
    start: usize,
    size: usize,
) -> anyhow::Result<Uuid> {
    let Some(bytes) = provider.get_blob_part(&id)? else {
        anyhow::bail!("unrecognized blob id {id}");
    };
    provider.create_blob_part(bytes.slice(start..(start + size)))
}

#[convex_macro::v8_op]
pub fn op_blob_read_part<'b, P: OpProvider<'b>>(
    provider: &mut P,
    id: Uuid,
) -> anyhow::Result<ToJsBuffer> {
    let Some(bytes) = provider.get_blob_part(&id)? else {
        anyhow::bail!("unrecognized blob id {id}");
    };
    Ok(bytes.to_vec().into())
}
