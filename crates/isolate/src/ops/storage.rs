use common::runtime::Runtime;
use deno_core::{
    serde_v8,
    v8::{
        self,
    },
};
use futures::channel::mpsc;

use crate::{
    environment::{
        AsyncOpRequest,
        IsolateEnvironment,
    },
    execution_scope::ExecutionScope,
    request_scope::StreamListener,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    pub fn async_op_storageStore(
        &mut self,
        args: v8::FunctionCallbackArguments,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let stream_id = serde_v8::from_v8(self, args.get(1))?;
        let (body_sender, body_receiver) = mpsc::unbounded();
        match stream_id {
            Some(stream_id) => {
                self.new_stream_listener(stream_id, StreamListener::RustStream(body_sender))?;
            },
            None => body_sender.close_channel(),
        };
        let content_type: Option<String> = serde_v8::from_v8(self, args.get(2))?;
        let content_type = content_type.filter(|ct| !ct.is_empty());
        let content_length = serde_v8::from_v8(self, args.get(3))?;
        let digest = serde_v8::from_v8(self, args.get(4))?;

        let state = self.state_mut();
        state.environment.start_async_op(
            AsyncOpRequest::StorageStore {
                body_stream: body_receiver,
                content_type,
                content_length,
                digest,
            },
            resolver,
        )
    }

    pub fn async_op_storageGet(
        &mut self,
        args: v8::FunctionCallbackArguments,
        resolver: v8::Global<v8::PromiseResolver>,
    ) -> anyhow::Result<()> {
        let storage_id = serde_v8::from_v8(self, args.get(1))?;

        let state = self.state_mut();
        let stream_id = state.create_stream()?;
        state.environment.start_async_op(
            AsyncOpRequest::StorageGet {
                storage_id,
                stream_id,
            },
            resolver,
        )
    }
}
