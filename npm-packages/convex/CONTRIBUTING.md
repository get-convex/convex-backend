# Contributing

Thanks for your interest in contributing to convex-js.

For anything not covered here, feel free to ask in the
[Convex Discord Community](https://convex.dev/community).

### I have a question

Great, please use GitHub discussions for this, or ask in Discord.

## I have a feature suggestion

Great, please open a GitHub issue on this repository for this or share in
Discord.

### I want to make a pull request

convex-js is developed primarily by employees of Convex Inc. We're excited to
provide transparency to our customers and contribute to the community by
releasing this code. We can accept some pull requests from non-employee
contributors, but please check in on Discord or in GitHub issues before getting
into anything more than small fixes to see if it's consistent with our short
term plan. We think carefully about how our APIs contribute to a cohesive
product, so chatting up front goes a long way.

Client tests can be run with

```
npm test
```

but be aware that there are integration tests, end-to-end tests, proptests, and
more which test this code but are not located in this repository.

# Directory structure notes

Code generally lives in the src/ directory.

There nearly-empty directories for each entry point at the top level implement
the 'package-json-redirects' strategy described at
https://github.com/andrewbranch/example-subpath-exports-ts-compat in an effort
to make the convex npm package as compatible as possible while making the
published package mirror the filesystem of this repository.
