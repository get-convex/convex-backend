import { ConvexError } from "convex/values";
import { useMutation } from "convex/react";
import { api } from "../convex/_generated/api";

export function MyApp() {
  const doSomething = useMutation(api.myFunctions.mutateSomething);
  const handleSomething = async () => {
    try {
      await doSomething({ a: 1, b: 2 });
    } catch (error) {
      const errorMessage =
        // Check whether the error is an application error
        error instanceof ConvexError
          ? // Access data and cast it to the type we expect
            (error.data as { message: string }).message
          : // Must be some developer error,
            // and prod deployments will not
            // reveal any more information about it
            // to the client
            "Unexpected error occurred";
      // do something with `errorMessage`
    }
  };
  // ...
}
