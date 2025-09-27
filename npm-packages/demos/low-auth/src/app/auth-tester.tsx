"use client";

import { useState, useEffect } from "react";
import { convex } from "./convex-client-provider";
import { useQuery } from "convex/react";
import { api } from "../../convex/_generated/api";

type AuthType = "none" | "plaintext" | "jwt";

interface AuthTesterProps {
  onAuthChange?: () => void;
}

export function AuthTester({ onAuthChange }: AuthTesterProps = {}) {
  const [authType, setAuthType] = useState<AuthType>("none");
  const [tokenValue, setTokenValue] = useState("");
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [authStatus, setAuthStatus] = useState("Not authenticated");

  const handleSetAuth = async () => {
    if (authType === "none") {
      convex.clearAuth();
      setAuthStatus("Authentication cleared");
      return;
    }

    if (!tokenValue.trim()) {
      alert("Please enter a token value");
      return;
    }

    const authChangeCallback = (authenticated: boolean) => {
      setIsAuthenticated(authenticated);
      setAuthStatus(authenticated ? `Authenticated with ${authType}` : `Failed to authenticate with ${authType}`);
      console.log(`Authentication status changed: ${authenticated} (${authType})`);
      // Trigger query refresh
      onAuthChange?.();
    };

    try {
      if (authType === "plaintext") {
        convex.setAuthInsecure(
          async () => tokenValue,
          authChangeCallback
        );
        setAuthStatus(`Setting plaintext auth...`);
      } else if (authType === "jwt") {
        convex.setAuth(
          async () => tokenValue,
          authChangeCallback
        );
        setAuthStatus(`Setting JWT auth...`);
      }
    } catch (error) {
      console.error("Error setting auth:", error);
      setAuthStatus(`Error: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleClearAuth = () => {
    convex.clearAuth();
    setIsAuthenticated(false);
    setAuthStatus("Authentication cleared");
    setTokenValue("");
    // Trigger query refresh
    onAuthChange?.();
  };

  const handleGenerateRandomToken = () => {
    const randomToken = crypto.randomUUID();
    setTokenValue(randomToken);
  };

  return (
    <div style={{ 
      marginTop: "30px", 
      padding: "20px", 
      border: "1px solid #ddd", 
      borderRadius: "8px",
      backgroundColor: "#f9f9f9"
    }}>
      <h3>Authentication Tester</h3>
      
      <div style={{ marginBottom: "15px" }}>
        <label htmlFor="authType" style={{ display: "block", marginBottom: "5px", fontWeight: "bold" }}>
          Authentication Type:
        </label>
        <select
          id="authType"
          value={authType}
          onChange={(e) => setAuthType(e.target.value as AuthType)}
          style={{ 
            padding: "8px", 
            borderRadius: "4px", 
            border: "1px solid #ccc",
            minWidth: "200px"
          }}
        >
          <option value="none">None (Clear Auth)</option>
          <option value="plaintext">Plaintext Token (setAuthInsecure)</option>
          <option value="jwt">JWT Token (setAuth)</option>
        </select>
      </div>

      {authType !== "none" && (
        <div style={{ marginBottom: "15px" }}>
          <label htmlFor="tokenInput" style={{ display: "block", marginBottom: "5px", fontWeight: "bold" }}>
            Token Value:
          </label>
          <div style={{ display: "flex", gap: "10px" }}>
            <input
              id="tokenInput"
              type="text"
              value={tokenValue}
              onChange={(e) => setTokenValue(e.target.value)}
              placeholder={authType === "jwt" ? "Enter JWT token..." : "Enter plaintext token..."}
              style={{ 
                flex: 1,
                padding: "8px", 
                borderRadius: "4px", 
                border: "1px solid #ccc"
              }}
            />
            {authType === "plaintext" && (
              <button
                onClick={handleGenerateRandomToken}
                style={{
                  padding: "8px 12px",
                  borderRadius: "4px",
                  border: "1px solid #007bff",
                  backgroundColor: "#007bff",
                  color: "white",
                  cursor: "pointer"
                }}
              >
                Random
              </button>
            )}
          </div>
        </div>
      )}

      <div style={{ marginBottom: "15px" }}>
        <button
          onClick={handleSetAuth}
          style={{
            padding: "10px 20px",
            marginRight: "10px",
            borderRadius: "4px",
            border: "1px solid #28a745",
            backgroundColor: "#28a745",
            color: "white",
            cursor: "pointer"
          }}
        >
          {authType === "none" ? "Clear Auth" : "Set Authentication"}
        </button>
        <button
          onClick={handleClearAuth}
          style={{
            padding: "10px 20px",
            borderRadius: "4px",
            border: "1px solid #dc3545",
            backgroundColor: "#dc3545",
            color: "white",
            cursor: "pointer"
          }}
        >
          Clear Auth
        </button>
      </div>

      <div style={{ 
        padding: "10px", 
        borderRadius: "4px", 
        backgroundColor: isAuthenticated ? "#d4edda" : "#f8d7da",
        border: `1px solid ${isAuthenticated ? "#c3e6cb" : "#f5c6cb"}`,
        color: isAuthenticated ? "#155724" : "#721c24"
      }}>
        <strong>Status:</strong> {authStatus}
      </div>
    </div>
  );
}