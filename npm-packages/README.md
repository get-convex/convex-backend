## About

npm-packages contains the cli, demos, tutorials, tests, scripts and more. We use
Rush for package management, so don't run `npm install` in these directories.
See the main convex/README.md file for details.

## Adding a new convex package

To add a new package that uses convex, follow these steps instead of our
quickstart:

1. Create a new subdirectory
2. Copy the contents of the npm-packages/tutorial into the subdirectory
3. Update `"name"` in your new `package.json` file to match your subdirectory
   name
4. Add your new package to
   [rush](https://github.com/get-convex/convex/blob/main/npm-packages/rush.json#L296)
5. Run `just rush update` to install dependencies

## Running a convex package

You can run projects against either a local backend, or against prod.

If your project requires changes that have not yet been deployed, you'll need to
test against a local backend.

To use a local backend:

1. Run `just run-backend`
2. In a separate terminal window, run `just convex dev`

To use a local big-brain (runs both backend and big brain locally):

1. Run `just run-big-brain`
2. Copy the `Test with:` line output by big-brain:

`2023-07-12T19:07:07.422400Z INFO big_brain::model: Test with: CONVEX_PROVISION_HOST=http://0.0.0.0:8050 npx convex dev --override-auth-url "https://convexdev-test.us.auth0.com/" --override-auth-client "XXXXXXXXX"`

3. Paste it into another terminal that's in your convex directory.

To run against prod:

Just run `npx convex dev`
