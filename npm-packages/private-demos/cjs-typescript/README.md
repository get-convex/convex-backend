This is a no-bundle TypeScript Node.js project which uses esbuild to compile the
index files.

It demonstrates

- properly typed imports of library code from module and commonjs TypeScript
  files with `"module": "node16"` and `"moduleResolution": "node"`
- runtime importing library code from compiled CJS JavaScript files
- CJS code generation via `npx convex codegen --commonjs`
- propertly typed imports of CJS codegen'd code
- runtime importing generated code `convex/_generated`

Using TypeScript to compile the project does not work: `tsc` compiles too many
files and changes the relative locations of convex/ functions.

Deploying functions with the command line requires extra steps: codegen may need
to be re-run `npx convex codegen --commonjs` after deploying. This project
cannot use `just convex dev` because that command will continue to generate
non-commonjs code.
