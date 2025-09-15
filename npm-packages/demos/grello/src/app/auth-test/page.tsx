"use client";

import { useQuery } from "convex/react";
import { api } from "../../../convex/_generated/api";
import { CONVEX_JWT_TOKEN_KEY } from "@/lib/convex";
import { useState, useEffect } from "react";

export default function AuthTestPage() {
  const currentUser = useQuery(api.query.getCurrentUserIdentity);
  const [currentToken, setCurrentToken] = useState<string | null>(null);
  const [tokenInput, setTokenInput] = useState("");
  const [isClient, setIsClient] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  useEffect(() => {
    setIsClient(true);
    const token = localStorage.getItem(CONVEX_JWT_TOKEN_KEY);
    setCurrentToken(token);
    setTokenInput(token || "");
  }, []);

  const handleSetToken = () => {
    if (!isClient) return;

    if (tokenInput.trim()) {
      localStorage.setItem(CONVEX_JWT_TOKEN_KEY, tokenInput.trim());
      setCurrentToken(tokenInput.trim());
      setFeedback("Token set successfully! Refresh the page to see changes.");
    } else {
      setFeedback("Please enter a token first.");
    }

    setTimeout(() => setFeedback(null), 3000);
  };

  const handleClearToken = () => {
    if (!isClient) return;

    localStorage.removeItem(CONVEX_JWT_TOKEN_KEY);
    setCurrentToken(null);
    setTokenInput("");
    setFeedback("Token cleared! Refresh the page to see changes.");

    setTimeout(() => setFeedback(null), 3000);
  };

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <h1 className="text-3xl font-bold mb-6">Auth Test Page</h1>

      <div className="space-y-4">
        <div className="bg-gray-50 p-4 rounded-lg">
          <h2 className="text-xl font-semibold mb-2">Current User Identity</h2>
          <div className="font-mono text-sm bg-white p-3 rounded border">
            {currentUser === undefined ? (
              <div className="text-blue-600">Loading...</div>
            ) : currentUser === null ? (
              <div className="text-red-600">Not authenticated</div>
            ) : (
              <pre className="text-green-600 whitespace-pre-wrap">
                {JSON.stringify(currentUser, null, 2)}
              </pre>
            )}
          </div>
        </div>

        <div className="bg-green-50 p-4 rounded-lg">
          <h2 className="text-xl font-semibold mb-2">Set JWT Token</h2>
          <div className="space-y-3">
            <textarea
              value={tokenInput}
              onChange={(e) => setTokenInput(e.target.value)}
              placeholder="Paste your JWT token here..."
              className="w-full p-3 border rounded-lg font-mono text-sm resize-none"
              rows={4}
            />
            <div className="flex gap-2">
              <button
                onClick={handleSetToken}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
              >
                Set Token
              </button>
              <button
                onClick={handleClearToken}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
              >
                Clear Token
              </button>
            </div>
            {feedback && (
              <div className="text-sm p-2 bg-white rounded border border-green-200 text-green-800">
                {feedback}
              </div>
            )}
          </div>
        </div>

        <div className="bg-blue-50 p-4 rounded-lg">
          <h2 className="text-xl font-semibold mb-2">JWT Token Status</h2>
          <div className="font-mono text-sm">
            <strong>localStorage key:</strong>{" "}
            <code>{CONVEX_JWT_TOKEN_KEY}</code>
            <br />
            <strong>Current token:</strong>
            <div className="mt-2 p-2 bg-white rounded border break-all">
              {isClient ? currentToken || "No token set" : "Loading..."}
            </div>
          </div>
        </div>

        <div className="bg-yellow-50 p-4 rounded-lg">
          <h2 className="text-xl font-semibold mb-2">Instructions</h2>
          <p>
            Use the input field above to easily set or clear your JWT token for
            testing authentication.
          </p>
          <p className="mt-2 text-sm text-gray-600">
            After setting a token, refresh the page to see the authentication
            result in the &quot;Current User Identity&quot; section.
          </p>
        </div>
      </div>
    </div>
  );
}
