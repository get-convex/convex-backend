import {
  QueryClient,
  QueryClientProvider,
  useMutation,
  useQuery,
} from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import {
  Authenticated,
  AuthLoading,
  ConvexReactClient,
  Unauthenticated,
} from "convex/react";
import ReactDOM from "react-dom/client";
import {
  ConvexQueryClient,
  convexAction,
  convexQuery,
  useConvexMutation,
} from "./index.js";
import "./index.css";
import { FormEvent, useState } from "react";
import { api } from "../convex/_generated/api.js";
import { ConvexAuthProvider, useAuthActions } from "@convex-dev/auth/react";
import { SuspenseMessageCountWithFallback } from "./suspense.js";

// Build a global convexClient wherever you would normally create a TanStack Query client.
const convexClient = new ConvexReactClient(import.meta.env.VITE_CONVEX_URL);
const convexQueryClient = new ConvexQueryClient(convexClient);
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // The queryKeyHashFn needs to be set globally: it cannot be specified
      // in `setQueryData()`, so the client couldn't update the query results.
      queryKeyHashFn: convexQueryClient.hashFn(),
      // The queryFn is convenient to set globally to avoid needing to import
      // the client everywhere.
      queryFn: convexQueryClient.queryFn(),
    },
  },
});
convexQueryClient.connect(queryClient);

function Main() {
  return (
    <ConvexAuthProvider client={convexClient}>
      <QueryClientProvider client={queryClient}>
        <AuthLoading>
          <div>Loading...</div>
        </AuthLoading>
        <Unauthenticated>
          <SignIn />
        </Unauthenticated>
        <Authenticated>
          <App />
        </Authenticated>
        <ReactQueryDevtools initialIsOpen />
      </QueryClientProvider>
    </ConvexAuthProvider>
  );
}

function SignIn() {
  const { signIn } = useAuthActions();
  const [step, setStep] = useState<"signUp" | "signIn">("signIn");
  return (
    <div className="signin-container">
      <form
        className="signin-form"
        onSubmit={(event) => {
          event.preventDefault();
          const formData = new FormData(event.currentTarget);
          void signIn("password", formData);
        }}
      >
        <input
          name="email"
          placeholder="Email"
          type="text"
          className="signin-input"
        />
        <input
          name="password"
          placeholder="Password"
          type="password"
          className="signin-input"
        />
        <input name="flow" type="hidden" value={step} />
        <button type="submit">
          {step === "signIn" ? "Sign in" : "Sign up"}
        </button>
        <button
          type="button"
          className="signin-secondary"
          onClick={() => {
            setStep(step === "signIn" ? "signUp" : "signIn");
          }}
        >
          {step === "signIn" ? "Sign up instead" : "Sign in instead"}
        </button>
      </form>
    </div>
  );
}

function Weather() {
  const { data, isPending, error } = useQuery(
    // This query doesn't update reactively, it refetches like a normal queryFn.
    convexAction(api.weather.getSFWeather),
  );
  if (isPending || error) return <span>?</span>;
  const fetchedAt = new Date(data.fetchedAt);
  return (
    <div className="weather">
      It is {data.fahrenheit}° F in San Francisco (fetched at{" "}
      {fetchedAt.toLocaleString("en-US", {
        hour: "numeric",
        minute: "2-digit",
        second: "2-digit",
      })}
      ).
    </div>
  );
}

function MessageCount() {
  const [shown, setShown] = useState(true);
  // This is a conditional query
  const { data, isPending, error } = useQuery(
    convexQuery(api.messages.count, shown ? {} : "skip"),
  );
  return (
    <div className="message-count">
      {isPending
        ? "? messages"
        : error
          ? "error counting messages"
          : `${data} messages`}
      <span onClick={() => setShown(!shown)}>
        {shown
          ? " (click to disable message count)"
          : " (click to enable message count)"}
      </span>
    </div>
  );
}

function SearchMessages() {
  const [searchTerm, setSearchTerm] = useState("");
  const { data, isPending } = useQuery(
    convexQuery(
      api.messages.search,
      searchTerm ? { query: searchTerm, limit: 5 } : "skip",
    ),
  );

  return (
    <div className="search-messages">
      <input
        type="text"
        placeholder="Search messages..."
        value={searchTerm}
        onChange={(e) => setSearchTerm(e.target.value)}
      />
      {isPending ? (
        <div>Searching...</div>
      ) : data && data.length > 0 ? (
        <ul>
          {data.map((message) => (
            <li key={message._id}>{message.body}</li>
          ))}
        </ul>
      ) : searchTerm ? (
        <div>No results found</div>
      ) : null}
    </div>
  );
}

function App() {
  const { signOut } = useAuthActions();

  const { data, error, isPending } = useQuery({
    // This query updates reactively.
    ...convexQuery(api.messages.list),
    initialData: [],
  });

  const {
    data: user,
    error: _userError,
    isPending: _userIsPending,
  } = useQuery({
    ...convexQuery(api.user.getCurrent),
    initialData: null,
  });

  const [newMessageText, setNewMessageText] = useState("");
  const { mutate, isPending: sending } = useMutation({
    mutationFn: useConvexMutation(api.messages.send),
  });
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    if (!user?._id) return;
    if (!sending && newMessageText) {
      mutate(
        { body: newMessageText, author: user._id },
        {
          onSuccess: () => setNewMessageText(""),
        },
      );
    }
  }
  if (error) {
    return <div>Error: {error.toString()}</div>;
  }
  if (isPending) {
    return <div>loading...</div>;
  }
  return (
    <main>
      <button type="button" onClick={() => void signOut()}>
        Sign out
      </button>
      <h1>Convex Chat</h1>
      <Weather />
      <MessageCount />
      <SuspenseMessageCountWithFallback />
      <SearchMessages />
      <p className="badge">
        <span>{user?.email}</span>
      </p>
      <ul>
        {data.map((message) => (
          <li key={message._id}>
            <span>{message.authorEmail}:</span>
            <span>{message.body}</span>
            <span>{new Date(message._creationTime).toLocaleTimeString()}</span>
          </li>
        ))}
      </ul>
      <form onSubmit={handleSendMessage}>
        <input
          value={newMessageText}
          onChange={(event) => setNewMessageText(event.target.value)}
          placeholder="Write a message…"
        />
        <input type="submit" value="Send" />
      </form>
    </main>
  );
}

const rootElement = document.getElementById("root")!;
ReactDOM.createRoot(rootElement).render(<Main />);
