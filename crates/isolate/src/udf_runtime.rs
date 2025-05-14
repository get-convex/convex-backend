use std::{
    borrow::Cow,
    sync::OnceLock,
};

use anyhow::Context as _;
use common::knobs::{
    ISOLATE_MAX_HEAP_EXTRA_SIZE,
    ISOLATE_MAX_USER_HEAP_SIZE,
};
use deno_core::{
    v8,
    ModuleSpecifier,
};

use crate::{
    bundled_js::system_udf_file,
    helpers,
    isolate::SETUP_URL,
    strings,
};

static BASE_SNAPSHOT: OnceLock<Vec<u8>> = OnceLock::new();

/// Creates a snapshot containing the UDF runtime in its default context and
/// saves it.
///
/// This must be called once per process, prior to calling
/// `create_isolate_with_udf_runtime`.
pub(crate) fn initialize() -> anyhow::Result<()> {
    let snapshot = create_base_snapshot()?.to_vec();
    BASE_SNAPSHOT
        .set(snapshot)
        .map_err(|_| anyhow::anyhow!("can't initialize more than once"))?;
    Ok(())
}

/// Set a 64KB initial heap size
const INITIAL_HEAP_SIZE: usize = 1 << 16;

/// Creates a new V8 isolate from the saved snapshot. Contexts created in this
/// isolate will have the UDF runtime already loaded.
pub(crate) fn create_isolate_with_udf_runtime() -> v8::OwnedIsolate {
    let snapshot = BASE_SNAPSHOT
        .get()
        .expect("udf_runtime::initialize not called");
    v8::Isolate::new(
        v8::CreateParams::default()
            .heap_limits(
                INITIAL_HEAP_SIZE,
                *ISOLATE_MAX_USER_HEAP_SIZE + *ISOLATE_MAX_HEAP_EXTRA_SIZE,
            )
            .snapshot_blob(Cow::Borrowed(&snapshot[..]).into()),
    )
}

fn create_base_snapshot() -> anyhow::Result<v8::StartupData> {
    // TODO: set external references. For now we:
    // 1. do not reuse snapshot blobs across processes,
    // 2. only use 'static external references
    // so this is OK.
    let mut isolate = v8::Isolate::snapshot_creator(None, None);

    let mut scope = v8::HandleScope::new(&mut isolate);

    let context = v8::Context::new(&mut scope, v8::ContextOptions::default());
    let mut context_scope = v8::ContextScope::new(&mut scope, context);

    // Create `global.Convex`, so that `setup.js` can populate `Convex.jsSyscall`
    let convex_value = v8::Object::new(&mut context_scope);
    let convex_key = strings::Convex.create(&mut context_scope)?;
    let global = context.global(&mut context_scope);
    global.set(&mut context_scope, convex_key.into(), convex_value.into());

    run_setup_module(&mut context_scope)?;

    drop(context_scope);
    // Mark the context we created as the "default context", so that every new
    // context created from the snapshot will include this runtime.
    scope.set_default_context(context);
    drop(scope);

    let data = isolate
        .create_blob(v8::FunctionCodeHandling::Keep)
        .context("Failed to create snapshot")?;

    Ok(data)
}

/// Go through all the V8 boilerplate to compile, instantiate, evaluate, and run
/// the setup code. This is all inlined to avoid any dependencies on context
/// state that isn't set up in the snapshot creation code path.
fn run_setup_module(scope: &mut v8::HandleScope<'_>) -> anyhow::Result<()> {
    let setup_url = ModuleSpecifier::parse(SETUP_URL)?;
    let (source, _source_map) = system_udf_file("setup.js").context("Setup module not found")?;
    let name_str =
        v8::String::new(scope, setup_url.as_str()).context("Failed to create name_str")?;
    let source_str = v8::String::new(scope, source).context("Failed to create source_str")?;
    let origin = helpers::module_origin(scope, name_str);
    let mut v8_source = v8::script_compiler::Source::new(source_str, Some(&origin));
    let module = v8::script_compiler::compile_module(scope, &mut v8_source)
        .context("Failed to compile setup module")?;

    // setup.js is bundled into a single module and so it doesn't need to import
    // anything.
    fn noop_resolve_module<'a>(
        _context: v8::Local<'a, v8::Context>,
        _specifier: v8::Local<'a, v8::String>,
        _import_assertions: v8::Local<'a, v8::FixedArray>,
        _referrer: v8::Local<'a, v8::Module>,
    ) -> Option<v8::Local<'a, v8::Module>> {
        None
    }
    anyhow::ensure!(
        module.instantiate_module(scope, noop_resolve_module) == Some(true),
        "Failed to instantiate setup module"
    );
    let evaluation_result = module.evaluate(scope).context("evaluate returned None")?;
    let status = module.get_status();
    anyhow::ensure!(
        status == v8::ModuleStatus::Evaluated,
        "Evaluating setup module failed with status {:?}",
        status
    );
    let promise = v8::Local::<v8::Promise>::try_from(evaluation_result)
        .context("Setup module didn't evaluate to a promise")?;
    anyhow::ensure!(
        promise.state() == v8::PromiseState::Fulfilled,
        "Setup module promise failed with state {:?}",
        promise.state()
    );

    let namespace = module
        .get_module_namespace()
        .to_object(scope)
        .context("Module namespace wasn't an object?")?;

    let function_str = strings::setup.create(scope)?;
    let function: v8::Local<v8::Function> = namespace
        .get(scope, function_str.into())
        .context("Missing setup function")?
        .try_into()?;

    let global = scope.get_current_context().global(scope);
    function
        .call(scope, global.into(), &[global.into()])
        .context("calling setup failed")?;
    Ok(())
}
