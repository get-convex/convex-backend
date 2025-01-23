# Simple HTML

Convex deployments can be connected to from JavaScript written without a
bundler. See index.html for an example.

Without API objects, Convex functions are referenced as strings:

- `"filename:myQuery"`
- `"directory/filename:myMutation"`
- `"directory/action:default"`

### typed-example.html and script.js

When you have the convex functions in the same repository you can use these
types using api objects. Without using a bundler it's necessary to annotate code
JSDoc comments to get autocompletion. See typed-example.html for an example.
