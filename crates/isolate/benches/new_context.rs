use deno_core::v8::{
    self,
    scope,
};
use isolate::ConcurrencyLimiter;
use runtime::prod::ProdRuntime;

fn main() {
    isolate::client::initialize_v8();
    divan::main();
}

#[divan::bench]
fn create_context(bencher: divan::Bencher) {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    let rt = ProdRuntime::new(&tokio_rt);
    let limiter = ConcurrencyLimiter::unlimited();
    let mut isolate = isolate::isolate::Isolate::new(rt.clone(), None, limiter.clone());
    bencher.bench_local(|| {
        scope!(let scope, isolate.isolate());
        v8::Context::new(scope, v8::ContextOptions::default());
    });
}

#[divan::bench]
fn create_isolate(bencher: divan::Bencher) {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    let rt = ProdRuntime::new(&tokio_rt);
    let limiter = ConcurrencyLimiter::unlimited();
    bencher.bench(|| {
        let mut isolate = isolate::isolate::Isolate::new(rt.clone(), None, limiter.clone());
        scope!(let scope, isolate.isolate());
        v8::Context::new(scope, v8::ContextOptions::default());
    });
}
