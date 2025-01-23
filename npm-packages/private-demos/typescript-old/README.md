Same as demos/typescript, but using older TypeScript settings. Currently the
oldest TypeScript version we support is the same as what's being used everywhere
else (TypeScript 5.0.4) but the tsconfig.json settings are different: in this
project both tsconfig.json files use "moduleResolution": "node", the older style
that doesn't understand package.json exports in libraries among other
shortcomings.
