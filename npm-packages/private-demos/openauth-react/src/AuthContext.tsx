import {
  useRef,
  useState,
  ReactNode,
  useEffect,
  useContext,
  createContext,
} from "react";
import { createClient } from "@openauthjs/openauth/client";

const client = createClient({
  clientID: "react",
  issuer: "http://localhost:3000",
});

interface AuthContextType {
  userId?: string;
  loaded: boolean;
  loggedIn: boolean;
  logout: () => void;
  login: () => Promise<void>;
  getToken: () => Promise<string | undefined>;
}

const AuthContext = createContext({} as AuthContextType);

export function AuthProvider({ children }: { children: ReactNode }) {
  const initializing = useRef(true);
  const [loaded, setLoaded] = useState(false);
  const [loggedIn, setLoggedIn] = useState(false);
  const token = useRef<string | undefined>(undefined);
  const [userId, setUserId] = useState<string | undefined>();

  useEffect(() => {
    const hash = new URLSearchParams(location.search.slice(1));
    const code = hash.get("code");
    const state = hash.get("state");

    if (!initializing.current) {
      return;
    }

    initializing.current = false;

    if (code && state) {
      callback(code, state);
      return;
    }

    auth();
    // example copied from https://github.com/toolbeam/openauth/tree/master/examples/client/react
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function auth() {
    const token = await refreshTokens();

    if (token) {
      await user();
    }

    setLoaded(true);
  }

  async function refreshTokens() {
    const refresh = localStorage.getItem("refresh");
    if (!refresh) return;
    const next = await client.refresh(refresh, {
      access: token.current,
    });
    if (next.err) return;
    if (!next.tokens) return token.current;

    localStorage.setItem("refresh", next.tokens.refresh);
    token.current = next.tokens.access;

    return next.tokens.access;
  }

  async function getToken() {
    const token = await refreshTokens();

    if (!token) {
      await login();
      return;
    }

    return token;
  }

  async function login() {
    const { challenge, url } = await client.authorize(location.origin, "code", {
      pkce: true,
    });
    sessionStorage.setItem("challenge", JSON.stringify(challenge));
    location.href = url;
  }

  async function callback(code: string, state: string) {
    const challenge = JSON.parse(sessionStorage.getItem("challenge")!);
    if (code) {
      if (state === challenge.state && challenge.verifier) {
        const exchanged = await client.exchange(
          code!,
          location.origin,
          challenge.verifier,
        );
        if (!exchanged.err) {
          token.current = exchanged.tokens?.access;
          localStorage.setItem("refresh", exchanged.tokens.refresh);
        }
      }
      window.location.replace("/");
    }
  }

  async function user() {
    const res = await fetch("http://localhost:3001/", {
      headers: {
        Authorization: `Bearer ${token.current}`,
      },
    });

    if (res.ok) {
      setUserId(await res.text());
      setLoggedIn(true);
    }
  }

  function logout() {
    localStorage.removeItem("refresh");
    token.current = undefined;

    window.location.replace("/");
  }

  return (
    <AuthContext.Provider
      value={{
        login,
        logout,
        userId,
        loaded,
        loggedIn,
        getToken,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}
