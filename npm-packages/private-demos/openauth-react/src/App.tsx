import { useState } from "react";
import { useAuth } from "./AuthContext";
import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

function App() {
  const auth = useAuth();
  const [status, setStatus] = useState("");

  async function callApi() {
    const res = await fetch("http://localhost:3001/", {
      headers: {
        Authorization: `Bearer ${await auth.getToken()}`,
      },
    });

    setStatus(res.ok ? "success" : "error");
  }

  return !auth.loaded ? (
    <div>Loading...</div>
  ) : (
    <div>
      {auth.loggedIn ? (
        <div>
          <p>
            <span>Logged in</span>
            {auth.userId && <span> as {auth.userId}</span>}
          </p>
          {status !== "" && <p>API call: {status}</p>}
          <ConvexData />
          <button onClick={callApi}>Call API</button>
          <button onClick={auth.logout}>Logout</button>
        </div>
      ) : (
        <button onClick={auth.login}>Login with OAuth</button>
      )}
    </div>
  );
}

function ConvexData() {
  const userInfo = useQuery(api.user.authInfo);
  return (
    <div>
      <code>
        <pre>
          User data in Convex: {JSON.stringify(userInfo || null, null, 2)}
        </pre>
      </code>
    </div>
  );
}

export default App;
