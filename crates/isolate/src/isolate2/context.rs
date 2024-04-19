use std::collections::BTreeMap;

use anyhow::anyhow;
use common::types::UdfType;
use deno_core::v8::{
    self,
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
            let context = v8::Context::new(&mut session.handle_scope);

            let mut handle_scope = v8::HandleScope::new(&mut session.handle_scope);
            let mut scope = v8::ContextScope::new(&mut handle_scope, context);

            let state = ContextState::new(environment);
            context.set_slot(&mut scope, state);

            let convex_value = v8::Object::new(&mut scope);

            let syscall_template = v8::FunctionTemplate::new(&mut scope, CallbackContext::syscall);
            let syscall_value = syscall_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let syscall_key = strings::syscall.create(&mut scope)?;
            convex_value.set(&mut scope, syscall_key.into(), syscall_value.into());

            let async_syscall_template =
                v8::FunctionTemplate::new(&mut scope, CallbackContext::async_syscall);
            let async_syscall_value = async_syscall_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let async_syscall_key = strings::asyncSyscall.create(&mut scope)?;
            convex_value.set(
                &mut scope,
                async_syscall_key.into(),
                async_syscall_value.into(),
            );

            let op_template = v8::FunctionTemplate::new(&mut scope, CallbackContext::op);
            let op_value = op_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let op_key = strings::op.create(&mut scope)?;
            convex_value.set(&mut scope, op_key.into(), op_value.into());

            let async_op_template =
                v8::FunctionTemplate::new(&mut scope, CallbackContext::start_async_op);
            let async_op_value = async_op_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let async_op_key = strings::asyncOp.create(&mut scope)?;
            convex_value.set(&mut scope, async_op_key.into(), async_op_value.into());

            let convex_key = strings::Convex.create(&mut scope)?;

            let global = context.global(&mut scope);
            global.set(&mut scope, convex_key.into(), convex_value.into());

            v8::Global::new(&mut scope, context)
        };

        let mut ctx = Self {
            context,
            next_function_id: 0,
            pending_functions: BTreeMap::new(),
        };
        ctx.enter(session, |mut ctx| ctx.run_setup_module())?;
        Ok(ctx)
    }

    pub fn enter<R>(&mut self, session: &mut Session, f: impl FnOnce(EnteredContext) -> R) -> R {
        let mut handle_scope = v8::HandleScope::new(&mut session.handle_scope);
        let context = v8::Local::new(&mut handle_scope, self.context.clone());
        let mut scope = v8::ContextScope::new(&mut handle_scope, context);
        let entered = EnteredContext::new(&mut scope, context);
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
