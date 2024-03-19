#![feature(try_blocks)]

use std::{
    sync::Arc,
    thread,
    time::Duration,
};

use crossbeam_channel::{
    bounded,
    Receiver,
};
use futures::{
    channel::oneshot,
    FutureExt,
};
use isolate::{
    client::initialize_v8,
    isolate2::{
        client::{
            AsyncSyscallCompletion,
            EvaluateResult,
            IsolateThreadClient,
            IsolateThreadRequest,
        },
        context::Context,
        environment::Environment,
        session::Session,
        thread::Thread,
    },
};
use serde_json::Value as JsonValue;
use tokio::sync::Semaphore;

fn v8_thread(
    receiver: Receiver<IsolateThreadRequest>,
    environment: Box<dyn Environment>,
) -> anyhow::Result<()> {
    let mut thread = Thread::new();
    let mut session = Session::new(&mut thread);
    let mut context = Context::new(&mut session, environment)?;

    match receiver.recv()? {
        IsolateThreadRequest::WaitForInitialized { response } => {
            let _ = response.send(());
        },
        _ => anyhow::bail!("Unexpected request"),
    }

    while let Ok(request) = receiver.recv() {
        match request {
            IsolateThreadRequest::RegisterModule {
                name,
                source,
                response,
            } => {
                let imports =
                    context.enter(&mut session, |mut ctx| ctx.register_module(&name, &source))?;
                let _ = response.send(imports);
            },
            IsolateThreadRequest::EvaluateModule { name, response } => {
                context.enter(&mut session, |mut ctx| ctx.evaluate_module(&name))?;
                let _ = response.send(());
            },
            IsolateThreadRequest::StartFunction {
                module,
                name,
                response,
            } => {
                let r = context.start_function(&mut session, &module, &name)?;
                let _ = response.send(r);
            },
            IsolateThreadRequest::PollFunction {
                function_id,
                completions,
                response,
            } => {
                let r = context.poll_function(&mut session, function_id, completions)?;
                let _ = response.send(r);
            },
            _ => anyhow::bail!("Unexpected request"),
        }
    }

    drop(context);
    drop(session);
    drop(thread);

    Ok(())
}

struct UdfEnvironment {}

impl Environment for UdfEnvironment {
    fn syscall(&mut self, op: &str, args: JsonValue) -> anyhow::Result<JsonValue> {
        if op == "echo" {
            return Ok(args);
        }
        anyhow::bail!("Syscall not implemented")
    }
}

async fn run_request(client: &mut IsolateThreadClient) -> anyhow::Result<String> {
    client.wait_for_initialized().await?;

    let requests = client
        .register_module(
            "convex:/foo.js".parse()?,
            r#"
            import { a, b, c } from './_deps/chunk.js';
            export async function f() { return await Convex.asyncSyscall('echo', '{"sup": 1}') }
            "#
            .to_string(),
        )
        .await?;
    println!("requests: {requests:?}");

    let requests = client
        .register_module(
            "convex:/_deps/chunk.js".parse()?,
            "export const a = 1; export const b = 2; export const c = 3;".to_string(),
        )
        .await?;
    println!("requests: {requests:?}");

    client.evaluate_module("convex:/foo.js".parse()?).await?;

    let (function_id, result) = client
        .start_function("convex:/foo.js".parse()?, "f".to_owned())
        .await?;
    let EvaluateResult::Pending { async_syscalls } = result else {
        anyhow::bail!("Unexpected result: {result:?}");
    };
    let mut completions = vec![];
    for async_syscall in async_syscalls {
        let promise_id = async_syscall.promise_id;
        let result = Ok(JsonValue::from(1));
        completions.push(AsyncSyscallCompletion { promise_id, result });
    }
    let result = client.poll_function(function_id, completions).await?;
    let EvaluateResult::Ready(result) = result else {
        anyhow::bail!("Unexpected result: {result:?}");
    };
    anyhow::Ok(result)
}

async fn tokio_thread(
    mut client: IsolateThreadClient,
    total_timeout: Duration,
    mut tx: oneshot::Sender<anyhow::Result<String>>,
) {
    let r = futures::select_biased! {
        r = run_request(&mut client).fuse() => r,

        // Eventually we'll attempt to cleanup the isolate thread in these conditions.
        _ = tokio::time::sleep(total_timeout).fuse() => Err(anyhow::anyhow!("Total timeout exceeded")),
        _ = tx.cancellation().fuse() => Err(anyhow::anyhow!("Cancelled")),
    };
    let _ = tx.send(r);
    drop(client);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_v8();
    let semaphore = Arc::new(Semaphore::new(8));
    let user_timeout = Duration::from_secs(5);

    // We actually don't really care about "system timeout" but rather "total
    // timeout", both for how long we're tying up a request thread + serving
    // based on a tx timestamp that may be out of retention.
    let total_timeout = Duration::from_secs(10);

    let environment = UdfEnvironment {};

    // The protocol is synchronous, so there should never be more than
    // one pending request at a time.
    let (sender, receiver) = bounded(1);
    let v8_handle = thread::spawn(|| {
        if let Err(e) = v8_thread(receiver, Box::new(environment)) {
            println!("Error in isolate thread: {:?}", e);
        }
    });
    let (tx, rx) = oneshot::channel();
    let client = IsolateThreadClient::new(sender, user_timeout, semaphore);
    let tokio_handle = tokio::spawn(tokio_thread(client, total_timeout, tx));

    let r = rx.await??;
    println!("Result: {}", r);

    tokio_handle.await?;
    v8_handle.join().unwrap();

    Ok(())
}
