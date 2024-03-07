use anyhow::Context;
use common::runtime::Runtime;
use deno_core::{
    JsBuffer,
    ToJsBuffer,
};

use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_blob_createPart(&mut self, bytes: JsBuffer) -> anyhow::Result<uuid::Uuid> {
        self.state_mut().create_blob_part(bytes.into())
    }

    #[convex_macro::v8_op]
    pub fn op_blob_slicePart(
        &mut self,
        id: uuid::Uuid,
        start: usize,
        size: usize,
    ) -> anyhow::Result<uuid::Uuid> {
        let state = self.state_mut();
        let Some(bytes) = state.blob_parts.get(&id).cloned() else {
            anyhow::bail!("unrecognized blob id {id}");
        };
        state.create_blob_part(bytes.slice(start..(start + size)))
    }

    #[convex_macro::v8_op]
    pub fn op_blob_readPart(&mut self, id: uuid::Uuid) -> anyhow::Result<ToJsBuffer> {
        let state = self.state_mut();
        let Some(bytes) = state.blob_parts.get(&id).cloned() else {
            anyhow::bail!("unrecognized blob id {id}");
        };
        Ok(bytes.to_vec().into())
    }
}
