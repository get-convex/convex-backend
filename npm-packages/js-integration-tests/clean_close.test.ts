import child_process from "child_process";

// Simple standalone scripts that just a UDF then close can keep Node.js
// process running from timers or the WebSocket connection not closing.
describe("JavaScript client closes cleanly", () => {
  test("Node.js subprocess exits quickly", () => {
    const p = child_process.spawnSync("node", ["clean_close_script.mjs"], {
      encoding: "utf-8",
      timeout: 5000,
    });
    expect(p.stderr).toEqual("");
    expect(p.stdout).toEqual("[]\n");
  });
});
