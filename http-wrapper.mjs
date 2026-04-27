/**
 * HTTP wrapper for the quality-gate MCP server.
 * Exposes the stdio JSON-RPC MCP protocol as a REST HTTP API
 * so it can run on Google Cloud Run.
 *
 * POST /mcp  → body: MCP JSON-RPC message → response: MCP JSON-RPC response
 * GET  /     → health check
 * GET  /status → server info + uptime
 */

import { createServer } from "http";
import { spawn } from "child_process";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

const __dirname = dirname(fileURLToPath(import.meta.url));

const PORT = parseInt(process.env.PORT || "8080", 10);
const MCP_BIN = join(__dirname, "servers/quality-gate/target/release/quality-gate-server");
const STARTED_AT = new Date().toISOString();

// Spawn MCP process (persistent, reused across requests)
let mcpProcess = null;
let pendingRequests = new Map(); // id → { resolve, reject, timer }
let buffer = "";

function startMcpProcess() {
  mcpProcess = spawn(MCP_BIN, [], {
    env: {
      ...process.env,
      WORKSPACE_MCP_HOME: process.env.WORKSPACE_MCP_HOME || __dirname,
      RETARGET_WORKSPACE: process.env.RETARGET_WORKSPACE || join(__dirname, "contracts"),
    },
    stdio: ["pipe", "pipe", "pipe"],
  });

  mcpProcess.stdout.on("data", (chunk) => {
    buffer += chunk.toString();
    const lines = buffer.split("\n");
    buffer = lines.pop(); // keep incomplete line
    for (const line of lines) {
      if (!line.trim()) continue;
      try {
        const msg = JSON.parse(line);
        const id = msg.id;
        if (id !== undefined && pendingRequests.has(id)) {
          const { resolve, timer } = pendingRequests.get(id);
          clearTimeout(timer);
          pendingRequests.delete(id);
          resolve(msg);
        }
      } catch (_) {}
    }
  });

  mcpProcess.stderr.on("data", (d) => {
    process.stderr.write("[mcp-stderr] " + d.toString());
  });

  mcpProcess.on("exit", (code) => {
    console.error(`[wrapper] MCP process exited with code ${code}. Restarting in 1s...`);
    mcpProcess = null;
    pendingRequests.forEach(({ reject, timer }) => {
      clearTimeout(timer);
      reject(new Error("MCP process died"));
    });
    pendingRequests.clear();
    setTimeout(startMcpProcess, 1000);
  });

  console.log("[wrapper] MCP process started, PID:", mcpProcess.pid);
}

function sendToMcp(message) {
  return new Promise((resolve, reject) => {
    if (!mcpProcess || mcpProcess.exitCode !== null) {
      return reject(new Error("MCP process not running"));
    }
    const id = message.id ?? Math.random().toString(36).slice(2);
    const msg = { ...message, id };
    const timer = setTimeout(() => {
      pendingRequests.delete(id);
      reject(new Error("MCP request timeout (30s)"));
    }, 30000);
    pendingRequests.set(id, { resolve, reject, timer });
    mcpProcess.stdin.write(JSON.stringify(msg) + "\n");
  });
}

// HTTP Server
const server = createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);

  // CORS headers
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, Authorization");

  if (req.method === "OPTIONS") {
    res.writeHead(204);
    return res.end();
  }

  // Health check
  if (req.method === "GET" && url.pathname === "/") {
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify({ status: "ok", service: "quality-gate-mcp" }));
  }

  // Status
  if (req.method === "GET" && url.pathname === "/status") {
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify({
      service: "quality-gate-mcp",
      version: "0.3.1",
      started_at: STARTED_AT,
      uptime_seconds: Math.floor((Date.now() - new Date(STARTED_AT).getTime()) / 1000),
      mcp_process_alive: mcpProcess !== null && mcpProcess.exitCode === null,
      pending_requests: pendingRequests.size,
    }));
  }

  // MCP proxy endpoint
  if (req.method === "POST" && url.pathname === "/mcp") {
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", async () => {
      try {
        const message = JSON.parse(body);
        const response = await sendToMcp(message);
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify(response));
      } catch (err) {
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: err.message }));
      }
    });
    return;
  }

  res.writeHead(404, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ error: "Not found" }));
});

startMcpProcess();
server.listen(PORT, () => {
  console.log(`[wrapper] HTTP server listening on port ${PORT}`);
  console.log(`[wrapper] MCP endpoint: POST http://localhost:${PORT}/mcp`);
  console.log(`[wrapper] Status: GET http://localhost:${PORT}/status`);
});
