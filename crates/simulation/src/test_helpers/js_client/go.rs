use std::sync::Arc;

use deno_core::{
    serde_v8,
    v8,
    ModuleSpecifier,
};
use isolate::{
    helpers::pump_message_loop,
    isolate::Isolate,
    ConcurrencyLimiter,
    RequestScope,
};
use runtime::testing::TestRuntime;
use tokio::sync::mpsc;

use super::{
    environment::TestEnvironment,
    state::JsThreadState,
    JsClientThread,
    JsClientThreadRequest,
};
use crate::test_helpers::{
    js_client::state::extract_error,
    server::ServerThread,
};

const TEST_SPECIFIER: &str = "convex:/test.js";

impl JsClientThread {
    pub async fn go(
        rt: TestRuntime,
        server: ServerThread,
        mut rx: mpsc::UnboundedReceiver<JsClientThreadRequest>,
    ) -> anyhow::Result<()> {
        let mut isolate = Isolate::new(rt.clone(), None, ConcurrencyLimiter::unlimited());
        let client_id = Arc::new(String::new());
        let environment = TestEnvironment::new(rt);
        let (handle, state) = isolate.start_request(client_id, environment).await?;
        let mut handle_scope = isolate.handle_scope();
        let v8_context = v8::Context::new(&mut handle_scope, v8::ContextOptions::default());
        let mut context_scope = v8::ContextScope::new(&mut handle_scope, v8_context);
        let mut isolate_context =
            RequestScope::new(&mut context_scope, handle.clone(), state, false).await?;

        {
            let mut v8_scope = isolate_context.scope();
            let mut scope = RequestScope::<TestRuntime, TestEnvironment>::enter(&mut v8_scope);
            let specifier = ModuleSpecifier::parse(TEST_SPECIFIER)?;
            let module = scope.eval_module(&specifier).await?;

            let mut state = JsThreadState::new(&mut scope, server, module)?;

            'main: loop {
                tracing::debug!("Processing inbox");
                state.process_js_inbox(&mut scope)?;

                tracing::debug!("Processing outbox");
                state.process_js_outbox(&mut scope)?;

                tracing::debug!("Performing microtask checkpoint");
                scope.perform_microtask_checkpoint();
                pump_message_loop(&mut scope);

                let rejections = scope.pending_unhandled_promise_rejections_mut();
                if let Some(promise) = rejections.exceptions.keys().next().cloned() {
                    let err = rejections.exceptions.remove(&promise).unwrap();
                    let err = v8::Local::new(&mut scope, err);
                    let err = extract_error(&mut scope, err)?;
                    anyhow::bail!(err);
                }

                // Don't block if we have something to do.
                // NB: We perform the microtask checkpoint last since we can't directly check
                // whether the microtask queue is empty.
                if !state.is_outbox_empty() || !state.is_inbox_empty() {
                    continue;
                }

                // Block, inject something into JS, and restart the loop.
                let isolate_state = scope.state_mut()?;
                let environment = &mut isolate_state.environment;
                tokio::select! {
                    maybe_req = rx.recv() => {
                        let Some(req) = maybe_req else {
                            break 'main;
                        };
                        state.handle_thread_request(&mut scope, req)?;
                    }
                    (web_socket_id, maybe_msg) = state.next_message() => {
                        state.handle_websocket_message(web_socket_id, maybe_msg)?;
                    }
                    resolver = environment.next_timer() => {
                        let resolver = resolver?;
                        let resolver = resolver.open(&mut scope);
                        let result = serde_v8::to_v8(&mut scope, ())?;
                        resolver.resolve(&mut scope, result);
                    }
                }
            }

            // TODO: Do this conditionally
            // state.print_replay_state();
        }
        tracing::info!("JsThread shutting down...");
        drop(isolate_context);
        handle.take_termination_error(None, "test")??;
        Ok(())
    }
}
