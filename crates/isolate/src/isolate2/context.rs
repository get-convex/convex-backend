use std::collections::BTreeMap;

use anyhow::{
    anyhow,
    Context as _,
};
use common::types::UdfType;
use deno_core::v8::{
    self,
    scope,
    scope_with_context,
};
use sync_types::CanonicalizedUdfPath;
use value::ConvexArray;

use super::{
    callback_context::CallbackContext,
    client::{
        Completions,
        EvaluateResult,
    },
    context_state::ContextState,
    entered_context::EnteredContext,
    environment::Environment,
    session::Session,
    FunctionId,
};
use crate::strings;

// Each isolate session can have multiple contexts, which we'll eventually use
// for subtransactions. Each context executes with a particular environment,
// and note that we could have different environments for different contexts.
pub struct Context {
    context: v8::Global<v8::Context>,

    next_function_id: FunctionId,
    pending_functions: BTreeMap<FunctionId, PendingFunction>,
}

impl Context {
    pub fn new(session: &mut Session, environment: Box<dyn Environment>) -> anyhow::Result<Self> {
        let context = {
            scope!(let handle_scope, session.isolate);
            let context = v8::Context::new(handle_scope, v8::ContextOptions::default());
            let mut scope = v8::ContextScope::new(handle_scope, context);

            let state = ContextState::new(environment);
            // TODO: this uses isolate-global slots, ideally it should use context-keyed
            // slots
            scope.set_slot(state);

            let global = context.global(&scope);
            let convex_key = strings::Convex.create(&scope)?;
            let convex_value: v8::Local<v8::Object> = global
                .get(&scope, convex_key.into())
                .context("Missing global.Convex")?
                .try_into()
                .context("Wrong type of global.Convex")?;

            let syscall_template = v8::FunctionTemplate::new(&scope, CallbackContext::syscall);
            let syscall_value = syscall_template
                .get_function(&scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let syscall_key = strings::syscall.create(&scope)?;
            convex_value.set(&scope, syscall_key.into(), syscall_value.into());

            let async_syscall_template =
                v8::FunctionTemplate::new(&scope, CallbackContext::async_syscall);
            let async_syscall_value = async_syscall_template
                .get_function(&scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let async_syscall_key = strings::asyncSyscall.create(&scope)?;
            convex_value.set(&scope, async_syscall_key.into(), async_syscall_value.into());

            let op_template = v8::FunctionTemplate::new(&scope, CallbackContext::op);
            let op_value = op_template
                .get_function(&scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let op_key = strings::op.create(&scope)?;
            convex_value.set(&scope, op_key.into(), op_value.into());

            let async_op_template =
                v8::FunctionTemplate::new(&scope, CallbackContext::start_async_op);
            let async_op_value = async_op_template
                .get_function(&scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let async_op_key = strings::asyncOp.create(&scope)?;
            convex_value.set(&scope, async_op_key.into(), async_op_value.into());

            v8::Global::new(&scope, context)
        };

        Ok(Self {
            context,
            next_function_id: 0,
            pending_functions: BTreeMap::new(),
        })
    }

    pub fn enter<R>(&mut self, session: &mut Session, f: impl FnOnce(EnteredContext) -> R) -> R {
        scope_with_context!(let scope, session.isolate, &self.context);
        let context = scope.get_current_context();
        let entered = EnteredContext::new(scope, &session.heap_context, context);
        f(entered)
    }

    pub fn start_function(
        &mut self,
        session: &mut Session,
        udf_type: UdfType,
        udf_path: CanonicalizedUdfPath,
        arguments: ConvexArray,
    ) -> anyhow::Result<(FunctionId, EvaluateResult)> {
        let function_id = self.next_function_id;
        self.next_function_id += 1;

        let (promise, result) = self.enter(session, |mut ctx| {
            ctx.start_evaluate_function(udf_type, &udf_path, arguments)
        })?;
        if let EvaluateResult::Pending { .. } = result {
            self.pending_functions
                .insert(function_id, PendingFunction { udf_path, promise });
        };
        Ok((function_id, result))
    }

    pub fn poll_function(
        &mut self,
        session: &mut Session,
        function_id: FunctionId,
        completions: Completions,
    ) -> anyhow::Result<EvaluateResult> {
        let Some(pending_function) = self.pending_functions.remove(&function_id) else {
            anyhow::bail!("Function {function_id} not found");
        };
        let result = self.enter(session, |mut ctx| {
            ctx.poll_function(&pending_function, completions)
        })?;
        if let EvaluateResult::Pending { .. } = result {
            self.pending_functions.insert(function_id, pending_function);
        }
        Ok(result)
    }
}

pub struct PendingFunction {
    pub udf_path: CanonicalizedUdfPath,
    pub promise: v8::Global<v8::Promise>,
}
