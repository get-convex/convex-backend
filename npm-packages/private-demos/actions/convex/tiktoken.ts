// // Note that directly importing form "@dqbd/tiktoken" doesn't work in either
// // Node nor v8 for different reasons:
// // - It doesn't work on v8, because `wasm.__wbindgen_add_to_stack_pointer is not a function`
// // import { get_encoding } from "@dqbd/tiktoken";
// // - It doesn't work on Node, because the package has different code path for Node,
// // that doesn't import the .wasm file and instead expects it to be present
// // on the file system. Imports of .wasm files are not allowed in Node in general.

// // Thus we follow the code examples here that manually import the wasm file
// // https://github.com/dqbd/tiktoken/tree/main/js#compatibility.

// import { init, Tiktoken } from "@dqbd/tiktoken/lite/init";
// import wasm from "../node_modules/@dqbd/tiktoken/lite/tiktoken_bg.wasm";
// // eslint-disable-next-line @typescript-eslint/ban-ts-comment
// // @ts-ignore importing json isn't support by our default tsconfig.json
// import model from "@dqbd/tiktoken/encoders/cl100k_base.json";
// import { action } from "./_generated/server";

// export default action(async (_, { text }: { text: string }) => {
//   console.log(wasm);
//   await init((imports) => WebAssembly.instantiate(wasm, imports));
//   const encoder = new Tiktoken(
//     model.bpe_ranks,
//     model.special_tokens,
//     model.pat_str
//   );
//   const tokens = encoder.encode(text);
//   encoder.free();
//   return `${tokens}`;
// });
