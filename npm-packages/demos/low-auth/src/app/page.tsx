"use client";

import { useQuery } from "convex/react";
import { api } from "../../convex/_generated/api";
import { AuthTester } from "./auth-tester";
import { useState } from "react";

export default function Page() {
  const [refreshKey, setRefreshKey] = useState(0);
  const result = useQuery(api.queries.getNumber, { refreshKey });

  if (result === undefined) {
    return <div>Loading...</div>;
  }

  return (
    <div style={{ padding: "20px", maxWidth: "800px", margin: "0 auto" }}>
      <h1>Hello, Next.js!</h1>
      
      <div style={{ marginBottom: "20px", padding: "15px", backgroundColor: "#f8f9fa", border: "1px solid #dee2e6", borderRadius: "6px" }}>
        <h3 style={{ marginTop: 0 }}>Query Results:</h3>
        {result.error ? (
          <p style={{ color: "red" }}><strong>Error:</strong> {result.error}</p>
        ) : (
          <>
            <p><strong>Number:</strong> {result.number}</p>
            <p><strong>Magic:</strong> <span style={{ color: result.magic ? "green" : "red", fontWeight: "bold" }}>{result.magic ? "true" : "false"}</span></p>
            <p><strong>getUserIdentity():</strong> {result.userIdentity ? JSON.stringify(result.userIdentity, null, 2) : "null"}</p>
            <p><strong>getUserIdentityInsecure():</strong> {result.userIdentityInsecure ? JSON.stringify(result.userIdentityInsecure, null, 2) : "null"}</p>
            <p><strong>getUserIdentityDebug():</strong> {result.userIdentityDebug ? JSON.stringify(result.userIdentityDebug, null, 2) : "null"}</p>
          </>
        )}
      </div>
      
      <div style={{ marginTop: "20px", padding: "10px", backgroundColor: "#e7f3ff", border: "1px solid #b3d9ff", borderRadius: "4px" }}>
        <strong>🧪 Development Mode:</strong> Test different authentication methods below
      </div>

      <AuthTester onAuthChange={() => setRefreshKey(prev => prev + 1)} />
      
      <div style={{ marginTop: "20px", padding: "10px", backgroundColor: "#f0f0f0", border: "1px solid #ccc", borderRadius: "4px" }}>
        <button 
          onClick={() => setRefreshKey(prev => prev + 1)}
          style={{
            padding: "8px 16px",
            borderRadius: "4px",
            border: "1px solid #007bff",
            backgroundColor: "#007bff",
            color: "white",
            cursor: "pointer"
          }}
        >
          Manual Refresh Query (Key: {refreshKey})
        </button>
      </div>
    </div>
  );
}
