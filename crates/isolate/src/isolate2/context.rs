use std::collections::HashMap;

use anyhow::anyhow;
use common::types::UdfType;
use deno_core::{
    v8::{
        self,
    },
    ModuleSpecifier,
};
use value::ConvexObject;

use super::{
    client::{
        AsyncSyscallCompletion,
        EvaluateResult,
    },
    context_state::ContextState,
    entered_context::EnteredContext,
    environment::Environment,
    session::Session,
    FunctionId,
};
use crate::{
    ops::run_op,
    strings,
};

// Each isolate session can have multiple contexts, which we'll eventually use
// for subtransactions. Each context executes with a particular environment,
// and note that we could have different environments for different contexts.
pub struct Context {
    context: v8::Global<v8::Context>,

    next_function_id: FunctionId,
    pending_functions: HashMap<FunctionId, v8::Global<v8::Promise>>,
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

            let syscall_template = v8::FunctionTemplate::new(&mut scope, Self::syscall);
            let syscall_value = syscall_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let syscall_key = strings::syscall.create(&mut scope)?;
            convex_value.set(&mut scope, syscall_key.into(), syscall_value.into());

            let async_syscall_template = v8::FunctionTemplate::new(&mut scope, Self::async_syscall);
            let async_syscall_value = async_syscall_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let async_syscall_key = strings::asyncSyscall.create(&mut scope)?;
            convex_value.set(
                &mut scope,
                async_syscall_key.into(),
                async_syscall_value.into(),
            );

            let op_template = v8::FunctionTemplate::new(&mut scope, Self::op);
            let op_value = op_template
                .get_function(&mut scope)
                .ok_or_else(|| anyhow!("Failed to retrieve function from FunctionTemplate"))?;
            let op_key = strings::op.create(&mut scope)?;
            convex_value.set(&mut scope, op_key.into(), op_value.into());

            let convex_key = strings::Convex.create(&mut scope)?;

            let global = context.global(&mut scope);
            global.set(&mut scope, convex_key.into(), convex_value.into());

            v8::Global::new(&mut scope, context)
        };

        let mut ctx = Self {
            context,
            next_function_id: 0,
            pending_functions: HashMap::new(),
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
        module: &ModuleSpecifier,
        name: &str,
        args: ConvexObject,
    ) -> anyhow::Result<(FunctionId, EvaluateResult)> {
        let function_id = self.next_function_id;
        self.next_function_id += 1;

        let (promise, result) = self.enter(session, |mut ctx| {
            ctx.start_evaluate_function(udf_type, module, name, args)
        })?;
        if let EvaluateResult::Pending { .. } = result {
            self.pending_functions.insert(function_id, promise);
        };
        Ok((function_id, result))
    }

    pub fn poll_function(
        &mut self,
        session: &mut Session,
        function_id: FunctionId,
        completions: Vec<AsyncSyscallCompletion>,
    ) -> anyhow::Result<EvaluateResult> {
        let Some(promise) = self.pending_functions.remove(&function_id) else {
            anyhow::bail!("Function {function_id} not found");
        };
        let result = self.enter(session, |mut ctx| ctx.poll_function(completions, &promise))?;
        if let EvaluateResult::Pending { .. } = result {
            self.pending_functions.insert(function_id, promise);
        }
        Ok(result)
    }

    pub fn syscall(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let mut ctx = EnteredContext::from_callback(scope);
        match ctx.syscall(args) {
            Ok(v) => rv.set(v),
            Err(_e) => {
                // XXX: Handle syscall or op error.
                // let message = strings::syscallError.create(scope).unwrap();
                // let exception = v8::Exception::error(scope, message);
                // scope.throw_exception(exception);
                todo!();
            },
        }
    }

    pub fn async_syscall(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
    ) {
        let mut ctx = EnteredContext::from_callback(scope);
        match ctx.start_async_syscall(args) {
            Ok(p) => rv.set(p.into()),
            Err(_e) => {
                // XXX: Handle syscall or op error.
                // let message = strings::syscallError.create(scope).unwrap();
                // let exception = v8::Exception::error(scope, message);
                // scope.throw_exception(exception);
                todo!();
            },
        }
    }

    pub fn op(
        scope: &mut v8::HandleScope,
        args: v8::FunctionCallbackArguments,
        rv: v8::ReturnValue,
    ) {
        let mut ctx = EnteredContext::from_callback(scope);
        if let Err(e) = run_op(&mut ctx, args, rv) {
            // XXX: Handle syscall or op error.
            // let message = strings::syscallError.create(scope).unwrap();
            // let exception = v8::Exception::error(scope, message);
            // scope.throw_exception(exception);
            panic!("Unexpected error: {e:?}");
        }
    }

    pub fn module_resolve_callback<'callback>(
        context: v8::Local<'callback, v8::Context>,
        specifier: v8::Local<'callback, v8::String>,
        _import_assertions: v8::Local<'callback, v8::FixedArray>,
        referrer: v8::Local<'callback, v8::Module>,
    ) -> Option<v8::Local<'callback, v8::Module>> {
        let mut scope = unsafe { v8::CallbackScope::new(context) };
        let mut ctx = EnteredContext::from_callback(&mut scope);
        ctx.resolve_module(specifier, referrer)
    }
}
