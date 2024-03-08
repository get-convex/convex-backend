use std::collections::BTreeMap;

use anyhow::Context;
use common::runtime::Runtime;
use deno_core::{
    serde_v8,
    v8::{
        self,
    },
    JsBuffer,
    ToJsBuffer,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    environment::{
        helpers::resolve_promise,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    request_scope::{
        ReadableStream,
        StreamListener,
    },
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_stream_create(&mut self) -> anyhow::Result<uuid::Uuid> {
        self.state_mut()?.create_stream()
    }

    #[convex_macro::v8_op]
    pub fn op_stream_extend(
        &mut self,
        id: uuid::Uuid,
        bytes: Option<JsBuffer>,
        new_done: bool,
    ) -> anyhow::Result<()> {
        self.extend_stream(id, bytes.map(|b| b.into()), new_done)
    }

    pub fn async_op_stream_readPart(
        &mut self,
        args: v8::FunctionCallbackArguments,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let stream_id = serde_v8::from_v8(self, args.get(1))?;
        self.new_stream_listener(stream_id, StreamListener::JsPromise(resolver))
    }

    pub fn new_stream_listener(
        &mut self,
        stream_id: uuid::Uuid,
        listener: StreamListener,
    ) -> anyhow::Result<()> {
        if self
            .state_mut()?
            .stream_listeners
            .insert(stream_id, listener)
            .is_some()
        {
            anyhow::bail!("cannot read from the same stream twice");
        }
        self.update_stream_listeners()
    }

    pub fn error_stream(&mut self, id: uuid::Uuid, error: anyhow::Error) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        state.streams.insert(id, Err(error));
        self.update_stream_listeners()
    }

    pub fn extend_stream(
        &mut self,
        id: uuid::Uuid,
        bytes: Option<bytes::Bytes>,
        new_done: bool,
    ) -> anyhow::Result<()> {
        let state = self.state_mut()?;
        let new_part_id = match bytes {
            Some(bytes) => Some(state.create_blob_part(bytes)?),
            None => None,
        };
        state.streams.mutate(&id, |stream| -> anyhow::Result<()> {
            let Some(Ok(ReadableStream { parts, done })) = stream else {
                anyhow::bail!("unrecognized stream id {id}");
            };
            if *done {
                anyhow::bail!("stream {id} is already done");
            }
            if let Some(new_part_id) = new_part_id {
                parts.push_back(new_part_id);
            }
            if new_done {
                *done = true;
            }
            Ok(())
        })?;
        self.update_stream_listeners()?;
        Ok(())
    }

    #[allow(unused)]
    pub fn create_complete_stream(&mut self, bytes: bytes::Bytes) -> anyhow::Result<uuid::Uuid> {
        let stream_id = self.state_mut()?.create_stream()?;
        self.extend_stream(stream_id, Some(bytes), false)?;
        self.extend_stream(stream_id, None, true)?;
        Ok(stream_id)
    }

    /// Call this when a stream has a new chunk or there is a new stream
    /// listener, to potentially notify the listeners.
    pub fn update_stream_listeners(&mut self) -> anyhow::Result<()> {
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct JsStreamChunk {
            done: bool,
            value: Option<ToJsBuffer>,
        }
        loop {
            let state = self.state_mut()?;
            let mut ready = BTreeMap::new();
            for stream_id in state.stream_listeners.keys() {
                let chunk = state.streams.mutate(
                    stream_id,
                    |stream| -> anyhow::Result<Result<(Option<Uuid>, bool), ()>> {
                        let stream = stream
                            .ok_or_else(|| anyhow::anyhow!("listening on nonexistent stream"))?;
                        let result = match stream {
                            Ok(stream) => Ok((stream.parts.pop_front(), stream.done)),
                            Err(_) => Err(()),
                        };
                        Ok(result)
                    },
                )?;
                match chunk {
                    Err(_) => {
                        ready.insert(
                            *stream_id,
                            Err(state.streams.remove(stream_id).unwrap().unwrap_err()),
                        );
                    },
                    Ok((chunk, stream_done)) => {
                        if let Some(chunk) = chunk {
                            let ready_chunk = state
                                .blob_parts
                                .remove(&chunk)
                                .ok_or_else(|| anyhow::anyhow!("stream chunk missing"))?;
                            ready.insert(*stream_id, Ok(Some(ready_chunk)));
                        } else if stream_done {
                            ready.insert(*stream_id, Ok(None));
                        }
                    },
                }
            }
            if ready.is_empty() {
                // Nothing to notify -- all caught up.
                return Ok(());
            }
            for (stream_id, update) in ready {
                if let Some(listener) = self.state_mut()?.stream_listeners.remove(&stream_id) {
                    match listener {
                        StreamListener::JsPromise(resolver) => {
                            let result = match update {
                                Ok(update) => Ok(serde_v8::to_v8(
                                    self,
                                    JsStreamChunk {
                                        done: update.is_none(),
                                        value: update.map(|chunk| chunk.to_vec().into()),
                                    },
                                )?),
                                Err(e) => Err(e),
                            };
                            resolve_promise(self, resolver, result)?;
                        },
                        StreamListener::RustStream(stream) => match update {
                            Ok(None) => stream.close_channel(),
                            Ok(Some(bytes)) => {
                                let _ = stream.unbounded_send(Ok(bytes));
                                self.state_mut()?
                                    .stream_listeners
                                    .insert(stream_id, StreamListener::RustStream(stream));
                            },
                            Err(e) => {
                                let _ = stream.unbounded_send(Err(e));
                                stream.close_channel();
                            },
                        },
                    }
                }
            }
        }
    }
}
