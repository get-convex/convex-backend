import React, { useState } from "react";
import CopyButton from "./CopyButton";

export function AgentLaunchCommand({
  command,
  dangerousFlag,
  prompt,
  dangerousLabel = "Give the agent full access (dangerous, skips all approval prompts)",
  defaultDangerous = false,
}: {
  command: string;
  dangerousFlag: string;
  prompt: string;
  dangerousLabel?: string;
  defaultDangerous?: boolean;
}) {
  const [dangerous, setDangerous] = useState(defaultDangerous);
  const cmd = dangerous
    ? `${command} ${dangerousFlag} "${prompt}"`
    : `${command} "${prompt}"`;
  return (
    <div className="agent-launch-command" style={{ marginBottom: "1.5rem" }}>
      <code className="convex-inline-code-with-copy-button">
        {cmd}
        <span className="convex-inline-code-copy-button">
          <CopyButton code={cmd} />
        </span>
      </code>
      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "0.5rem",
          marginTop: "0.5rem",
          fontSize: "0.9rem",
        }}
      >
        <input
          type="checkbox"
          checked={dangerous}
          onChange={(e) => setDangerous(e.target.checked)}
        />
        <span>{dangerousLabel}</span>
      </label>
    </div>
  );
}
