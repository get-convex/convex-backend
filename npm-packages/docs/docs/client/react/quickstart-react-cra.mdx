---
title: Create-React-App Quickstart
sidebar_label: Create React App
description: "Add Convex to a Create React App project"
slug: "quickstart-create-react-app"
hide_table_of_contents: true
sidebar_position: 1000
---

Learn how to query data from Convex in a React app using Create React App.

Alternatively check out the [React Quickstart](/quickstart/react.mdx) using
Vite.

<StepByStep>
  <Step title="Create a React app">
    Create a React app using the `create-react-app` command.

    ```sh
    npx create-react-app my-app
    ```

  </Step>
  <Step title="Install the Convex client and server library">
    To get started, install the `convex`
    package which provides a convenient interface for working
    with Convex from a React app.

    Navigate to your app directory and install `convex`.


    ```sh
    cd my-app && npm install convex
    ```

  </Step>
  <Step title="Set up a Convex dev deployment">
    Next, run `npx convex dev`. This
    will prompt you to log in with GitHub,
    create a project, and save your production and deployment URLs.

    It will also create a `src/convex/` folder for you
    to write your backend API functions in. The `dev` command
    will then continue running to sync your functions
    with your dev deployment in the cloud.


    ```sh
    npx convex dev
    ```

  </Step>

  <Step title="Create sample data for your database">
    In a new terminal window, create a `sampleData.jsonl`
    file with some sample data.

    ```csv title="sampleData.jsonl"
    {"text": "Buy groceries", "isCompleted": true}
    {"text": "Go for a swim", "isCompleted": true}
    {"text": "Integrate Convex", "isCompleted": false}
    ```

  </Step>

  <Step title="Add the sample data to your database">
    Now that your project is ready, add a `tasks` table
    with the sample data into your Convex database with
    the `import` command.

    ```
    npx convex import --table tasks sampleData.jsonl
    ```

  </Step>

  <Step title="Expose a database query">
    Add a new file `tasks.js` in the `src/convex/` folder
    with a query function that loads the data.

    Exporting a query function from this file
    declares an API function named after the file
    and the export name, `api.tasks.get`.

    ```js title="src/convex/tasks.js"
    import { query } from "./_generated/server";

    export const get = query({
      args: {},
      handler: async (ctx) => {
        return await ctx.db.query("tasks").collect();
      },
    });
    ```

  </Step>

  <Step title="Connect the app to your backend">
    In `index.js`, create a `ConvexReactClient` and pass it to a `ConvexProvider`
    wrapping your app.

    ```js title="src/index.js"
    import { ConvexProvider, ConvexReactClient } from "convex/react";

    const convex = new ConvexReactClient(process.env.REACT_APP_CONVEX_URL);
    root.render(
      <React.StrictMode>
        <ConvexProvider client={convex}>
          <App />
        </ConvexProvider>
      </React.StrictMode>
    );
    ```

  </Step>

  <Step title="Display the data in your app">
      In `App.js`, use the `useQuery` hook to fetch from your `api.tasks.get`
      API function.

      ```js title="src/App.js"
      import { useQuery } from "convex/react";
      import { api } from "./convex/_generated/api";

      function App() {
        const tasks = useQuery(api.tasks.get);
        return (
          <div className="App">
            {JSON.stringify(tasks, null, 2)}
          </div>
        );
      }
      ```

  </Step>

  <Step title="Start the app">
      Start the app, go to [http://localhost:3000](http://localhost:3000) in a browser,
      and see the serialized list of tasks at the top of the page.

      ```sh
      npm start
      ```

  </Step>

</StepByStep>
