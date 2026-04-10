import { SignInButton, UserButton } from "@clerk/react";
import { Authenticated, Unauthenticated, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

function App() {
  return (
    <main>
      <Unauthenticated>
        <SignInButton />
      </Unauthenticated>
      <Authenticated>
        <UserButton />
        <Content />
      </Authenticated>
    </main>
  );
}

function Content() {
  const messages = useQuery({
    query: api.messages.getForCurrentUser,
    args: {},
    throwOnError: true,
  }).data;
  return <div>Authenticated content: {messages?.length}</div>;
}

export default App;
