## React Native testing

Since we don't usually build React Native apps, we want to make sure we're still exercising
all the relevant code paths periodically so we don't accidentally make client changes that break them (e.g. using an API that's unavailable in React Native).

There are tests that run periodically in CI via `.github/workflows/react_native.yml`. A test run can be kicked off manually on a provided branch by clicking "Run workflow"
on [this page](https://github.com/get-convex/convex/actions/workflows/react_native.yml).

Below are the instructions for debugging these test failures or manually QAing React Native.

## Debugging test failures

Here's the recommended flow for debugging test failures:
* Manually QA a React Native app to see if React Native support is actually broken
    * If so, fix the bug in our library, and re-run the test to confirm. Make sure we don't do an NPM release while React Native support is broken.
    * If not, run the test locally to see if it fails there
        * If so, great! We can iterate locally and try and fix any issues with the test setup.
        * If not, there's an issue that's specific to the GitHub Actions environment, so we have no choice other than debugging in the action.

To test any changes in CI, push up a branch and trigger a test run manually by clicking "Run workflow" on [this page](https://github.com/get-convex/convex/actions/workflows/react_native.yml).

## Manually QAing React Native

We'll be using Expo to run a React Native app locally since it's easiest to set up.

Either clone the public demo from [here](https://github.com/get-convex/convex-demos/tree/main/react-native) or [here](/npm-packages/demos/react-native), or follow the [quickstart](https://docs.convex.dev/quickstart/react-native) to get a simple app with React Native + Convex.

It's probably easiest to do this outside of the monorepo -- if doing this inside the monorepo, use `npm install` instead of `rush` commands.

To run this with the version of Convex in the monorepo, run
```
python3 link_with_local.py <absolute path to monorepo root> --demo-relative-path <relative path to your app from the monorepo root>
```

Alternatively, create an alpha release for Convex ([instructions](/ops/services/npm/release.md)) and then install it with
```
npm install convex@alpha
```

Spin up the app and make sure you can load data + execute mutations.

## Running the test locally

There are two main ways to build a React Native app:
* Using Expo (see `npm-packages/demos/react-native`)
* Using `npx react-native init` (this app)

Expo is generally easier to use, and is what we show in our public demo and quickstart.

This private demo uses the latter and exists so we can run end to end tests against it using detox.

Only do this step if you have **already tried manually QAing the public demo**, the public demo works, but the test is failing.

#### Environment Setup

Follow the guide at [React Native guide](https://reactnative.dev/docs/set-up-your-environment) (specifically macOS + iOS).

Note important steps:

1. `brew install watchman`
2. Install Xcode
3. Install Xcode Command Line Tools
4. Install an iOS simulator
5. `sudo gem install cocoapods`
6. `cd ios`, `bundle install`, `bundle exec pod install`

#### Running the test
In one terminal from the monorepo root:

```
just run-backend
```

In a separate terminal within the react-native directory:

```
python3 link_with_local.py <absolute path to monorepo root>
python3 run_tests.py <absolute path to monorepo root>
```

### SSH-ing into a GitHub Actions runner

**Only do this step if you have tried everything else and the test is still failing.**

If the test is failing but the logs are not giving us enough information, we can
`ssh` into the machine that the test is running on using [this action](https://github.com/mxschmitt/action-tmate).

Add this step before whichever step is failing, push up a branch, and trigger a test run manually by clicking "Run workflow" on [this page](https://github.com/get-convex/convex/actions/workflows/react_native.yml).

Wait until the test reaches that step and a command to `ssh` into the machine is printed.


#### Troubleshooting

If the build command fails and you don't see the error message, run the command
`npx detox build --configuration ios.sim.release` directly to see all logs.

If you get the error `'value' is unavailable: introduced in iOS 12.0`
then (following [these instructions](https://github.com/facebook/react-native/issues/34106))
you should modify `node_modules/react-native/scripts/cocoapods/codegen_utils.rb`
to replace `'ios' => '11.0'` with `'ios' => '12.0'`.

If you get the error `applesimutils: command not found` then (following
[these instructions](https://github.com/wix/AppleSimulatorUtils/blob/master/README.md#installing)) you should run `brew tap wix/brew`, `brew install applesimutils`.

