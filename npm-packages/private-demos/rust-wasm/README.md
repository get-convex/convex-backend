## WebAssembly notes

Runtimes and bundlers use WebAssembly to produce EcmaScript modules (ESM) in two
different ways.

You've always been able to get the bytes and load them yourself; nothing to do
with ESM here.

```js
// simple, not streaming, not encouraged in browsers
const resp = await fetch("simple.wasm");
const bytes = await resp.arrayBuffer();
const module = WebAssembly.compile(bytes))
const instance = await WebAssembly.instantiate(module, imports)
instance.exports.foo(1, 2, 3);

// fancier, recommended in browsers
const resp = await fetch("simple.wasm");
const instance = WebAssembly.instantiateStreaming(resp, imports);
instance.exports.foo(1, 2, 3);
```

These APIs work everywhere and they've been enough to accomplish a lot with
WebAssembly.

Whenever WebAssembly is used the modules must be compiled into instances and
those instances must be "linked," provided its dependencies in the form of an
object of functions.

```js
const imports = {
  dep1: (a) => a + 1,
  dep2: () => Date.now(),
};
const instance = await WebAssembly.instantiate(module, imports);
instance.exports.foo(1, 2, 3);
```

This linking and instantiation work sounds like a job for modules! How should
they work?

### Instance

The semantics
[originally proposed](https://github.com/WebAssembly/esm-integration/tree/main/proposals/esm-integration)
by WASM folks like Andreas Rossberg are now in stage 2.

Just import a module and it's already instantiated and linked for you!

```js
import * as foo from "./foo.wasm";
foo.bar();
```

This style [has been implemented](https://github.com/nodejs/node/pull/27659)
behind the `--experimental-wasm-modules` flag in Node.js for years, since 12.3
in May of 2019.

You can try this in Node today:

```
$ node --experimental-wasm-modules
> foo = await("foo.wasm")
> foo.bar()
```

This "[asyncWebAssembly](https://webpack.js.org/configuration/experiments/)"
option can be enabled in Webpack will be the default behavior in Webpack 6. We
use this in the dashboard to import Cloudflare's cron parsing library.

The Rust toolchain [wasm-pack](https://rustwasm.github.io/wasm-pack/) generates
code and types that assume this behavior.

There's an
[esbuild plugin that does this](https://github.com/Tschrock/esbuild-plugin-wasm).

There's no movement from V8 on this but WebKit
[has it working behind a flag](https://bugs.webkit.org/show_bug.cgi?id=236268).

### Module

It's been [raised](https://github.com/WebAssembly/esm-integration/issues/14),
[several](https://github.com/WebAssembly/esm-integration/issues/44)
[times](https://github.com/WebAssembly/esm-integration/issues/63) that it's
sometimes useful to work with WebAssembly modules rather than instances.

So what if importing a WebAssembly modules didn't do the linking and
instantiating?

```js
import module from "./foo.wasm";
const instance = WebAssembly.instantiate(module);
instance.exports.bar();
```

This is Cloudflare's
[default behavior](https://developers.cloudflare.com/workers/wrangler/bundling/).

This is what we Convex implements implement with our plugin. We did this to
support @dqbd/tiktoken. In our PR implementing this behavior it's mentioned that
Next works like this, I'm having trouble finding reference to this. In some
bundlers (citation needed) appending the query parameter `?module` to the import
provides this behavior.

A [new proposal](https://github.com/tc39/proposal-source-phase-imports) for
"Source Phase Import" allows explicitly importing the module object. Switching
to this syntax for module object imports will disambiguate standard import
syntax.

```
import source FooModule from "./foo.wasm";
FooModule instanceof WebAssembly.Module; // true
```

## Did we implement the wrong one?

It seems like it, but we have latitude to fix it. We just need to keep
supporting the npm packages we already support. Currently WebAssembly support is
almost certainly only being used in Node.js.

We could support both with configuration, or without configuration we could
require the ?module suffix.

> Question
>
> How do bundlers implement this, do we need to support dynamic imports?

Langchain no longer uses the WebAssembly version of Tiktoken, it now uses
https://www.npmjs.com/package/js-tiktoken

Presumably if we wait long enough V8 will implement this natively.

The cost to our developer users of not implementing the instance approach isn't
clear. I'm annoyed I can't use wasm-bindgen/wasm-pack from Rust, but no customer
has asked for that.

### wasm-bindgen workaround

Until Convex supports bundling this way it's possible to

- wasm-pack build --target bundler // (this is the default target)
- TODO
