#!/usr/bin/env tsx
import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import * as repl from "repl";
import * as readline from "readline";
import * as dotenv from "dotenv";
import * as path from "path";
import * as fs from "fs";

dotenv.config({ path: path.join(__dirname, ".env.local") });

const CONVEX_URL = process.env.NEXT_PUBLIC_CONVEX_URL;

if (!CONVEX_URL) {
  console.error("❌ NEXT_PUBLIC_CONVEX_URL not found in .env.local");
  console.error("Please ensure your .env.local file contains:");
  console.error("NEXT_PUBLIC_CONVEX_URL=https://your-deployment.convex.cloud");
  process.exit(1);
}

console.log("🚀 Initializing Convex TypeScript REPL...");
console.log(`📡 Connecting to: ${CONVEX_URL}`);

const client = new ConvexReactClient(CONVEX_URL);

// Subscription management
const subscriptions = new Map<number, () => void>(); // Map of subscription ID to unsubscribe function
let subscriptionCounter = 0;

// History management configuration
const HISTORY_FILE = path.join(__dirname, ".convex_repl_history");
const HISTORY_SIZE = 1000;

// Load history from file
const loadHistory = (): string[] => {
  try {
    if (fs.existsSync(HISTORY_FILE)) {
      const historyData = fs.readFileSync(HISTORY_FILE, "utf-8");
      return historyData
        .split("\n")
        .filter((line) => line.trim().length > 0)
        .slice(-HISTORY_SIZE); // Keep only the last HISTORY_SIZE entries
    }
  } catch (error) {
    console.warn("⚠️  Warning: Could not load REPL history:", error.message);
  }
  return [];
};

// Save history to file
const saveHistory = (history: string[]) => {
  try {
    const historyToSave = history.slice(-HISTORY_SIZE); // Keep only last HISTORY_SIZE entries
    fs.writeFileSync(HISTORY_FILE, historyToSave.join("\n") + "\n", "utf-8");
  } catch (error) {
    console.warn("⚠️  Warning: Could not save REPL history:", error.message);
  }
};

// Clear history file
const clearHistory = () => {
  try {
    if (fs.existsSync(HISTORY_FILE)) {
      fs.unlinkSync(HISTORY_FILE);
      console.log("🗑️  REPL history cleared");
    } else {
      console.log("ℹ️  No history file to clear");
    }
  } catch (error) {
    console.warn("⚠️  Warning: Could not clear history:", error.message);
  }
};

// Subscription helper functions
const formatSubscriptionData = (data: any): string => {
  if (data === null || data === undefined) {
    return String(data);
  }
  
  if (Array.isArray(data)) {
    return `[\n${data.map((item, index) => `  [${index}] ${JSON.stringify(item, null, 2).split('\n').join('\n  ')}`).join('\n')}\n]`;
  }
  
  return JSON.stringify(data, null, 2);
};

const defaultSubscriptionCallback = (subscriptionId: number, queryName: string) => {
  return (data: any) => {
    const timestamp = new Date().toLocaleTimeString();
    console.log(`\n📡 [${subscriptionId}] ${queryName} @ ${timestamp}`);
    console.log(`━`.repeat(60));
    
    if (Array.isArray(data)) {
      console.log(`📊 ${data.length} rows returned:`);
      if (data.length > 0) {
        console.log(formatSubscriptionData(data));
      } else {
        console.log("   (empty result set)");
      }
    } else {
      console.log("📄 Result:");
      console.log(formatSubscriptionData(data));
    }
    console.log(`━`.repeat(60));
    
    // Re-show the prompt for better UX
    if ((r as any).displayPrompt) {
      (r as any).displayPrompt();
    }
  };
};

// API function listing using Convex function-spec
interface ConvexFunction {
  name: string;
  module: string;
  type: 'query' | 'mutation' | 'action';
  fullPath: string;
  args: string[];
}

const getConvexFunctions = async (): Promise<ConvexFunction[]> => {
  try {
    // Use the bash command to get function specs
    const { execSync } = require('child_process');
    const result = execSync('npx convex function-spec', { encoding: 'utf-8' });
    const spec = JSON.parse(result);
    
    return spec.functions.map((func: any) => {
      // Parse identifier like "users.js:createUser" 
      const [moduleFile, functionName] = func.identifier.split(':');
      const moduleName = moduleFile.replace('.js', '');
      
      // Extract argument names from the function spec
      const args = extractArgsFromSpec(func.args);
      
      return {
        name: functionName,
        module: moduleName,
        type: func.functionType.toLowerCase() as 'query' | 'mutation' | 'action',
        fullPath: `${moduleName}.${functionName}`,
        args: args
      };
    });
  } catch (error) {
    console.warn('⚠️  Could not get function specs, falling back to known functions');
    // Fallback to known functions if function-spec fails
    return [
      { name: 'queryIdentity', module: 'auth', type: 'query', fullPath: 'auth.queryIdentity', args: [] },
      { name: 'checkIdentity', module: 'auth', type: 'action', fullPath: 'auth.checkIdentity', args: [] },
      { name: 'createUser', module: 'users', type: 'mutation', fullPath: 'users.createUser', args: ['email', 'name'] },
      { name: 'getUserByEmail', module: 'users', type: 'query', fullPath: 'users.getUserByEmail', args: ['email'] },
      { name: 'getUserById', module: 'users', type: 'query', fullPath: 'users.getUserById', args: ['userId'] },
      { name: 'allCounters', module: 'counter', type: 'query', fullPath: 'counter.allCounters', args: [] }
    ];
  }
};

const extractArgsFromSpec = (argsSpec: any): string[] => {
  if (!argsSpec || argsSpec.type !== 'object' || !argsSpec.value) {
    return [];
  }
  
  return Object.keys(argsSpec.value).map(key => {
    const field = argsSpec.value[key];
    const isOptional = field.optional;
    return isOptional ? `${key}?` : key;
  });
};

const formatFunctionList = (functions: ConvexFunction[], filter?: string): void => {
  let filteredFunctions = functions;
  
  // Apply filter if provided
  if (filter) {
    const filterLower = filter.toLowerCase();
    filteredFunctions = functions.filter(f => 
      f.type === filterLower || 
      f.module.includes(filterLower) || 
      f.name.toLowerCase().includes(filterLower) ||
      f.fullPath.toLowerCase().includes(filterLower)
    );
  }
  
  if (filteredFunctions.length === 0) {
    console.log(filter ? `📭 No functions found matching "${filter}"` : '📭 No functions found');
    return;
  }
  
  // Group by type
  const grouped = {
    query: filteredFunctions.filter(f => f.type === 'query'),
    mutation: filteredFunctions.filter(f => f.type === 'mutation'),
    action: filteredFunctions.filter(f => f.type === 'action')
  };
  
  console.log('\n📋 Available Convex Functions');
  if (filter) {
    console.log(`🔍 Filtered by: "${filter}"`);
  }
  console.log(`━`.repeat(50));
  
  // Display each type
  if (grouped.query.length > 0) {
    console.log(`\n🔍 QUERIES (${grouped.query.length} function${grouped.query.length !== 1 ? 's' : ''})`);
    displayFunctionGroup(grouped.query);
  }
  
  if (grouped.mutation.length > 0) {
    console.log(`\n✏️  MUTATIONS (${grouped.mutation.length} function${grouped.mutation.length !== 1 ? 's' : ''})`);
    displayFunctionGroup(grouped.mutation);
  }
  
  if (grouped.action.length > 0) {
    console.log(`\n⚡ ACTIONS (${grouped.action.length} function${grouped.action.length !== 1 ? 's' : ''})`);
    displayFunctionGroup(grouped.action);
  }
  
  const total = filteredFunctions.length;
  console.log(`\n📊 Total: ${total} function${total !== 1 ? 's' : ''} available`);
  console.log(`💡 Usage: query(api.module.function, args) | mutation(api.module.function, args) | action(api.module.function, args)`);
  console.log(`🔔 Subscribe: subscribe(api.module.query, args, callback?)`);
  console.log(`━`.repeat(50));
};

const displayFunctionGroup = (functions: ConvexFunction[]): void => {
  // Group by module for cleaner display
  const byModule = functions.reduce((acc, func) => {
    const module = func.module || 'root';
    if (!acc[module]) acc[module] = [];
    acc[module].push(func);
    return acc;
  }, {} as Record<string, ConvexFunction[]>);
  
  for (const [module, funcs] of Object.entries(byModule)) {
    console.log(`  ${module}/`);
    funcs.forEach((func, index) => {
      const isLast = index === funcs.length - 1;
      const prefix = isLast ? '    └──' : '    ├──';
      const argsDisplay = func.args && func.args.length > 0 ? `(${func.args.join(', ')})` : '()';
      console.log(`${prefix} ${func.name}${argsDisplay}`);
    });
  }
};

// Setup JWT authentication from .jwtSession file
const setupAuth = () => {
  client.setAuth(async () => {
    try {
      const jwtSessionPath = path.join(__dirname, ".jwtSession");

      // Check if .jwtSession file exists
      if (!fs.existsSync(jwtSessionPath)) {
        return null; // No authentication
      }

      // Read JWT token from .jwtSession file
      const jwtToken = fs.readFileSync(jwtSessionPath, "utf-8");

      // Clean the token (remove newlines, whitespace)
      const cleanToken = jwtToken.trim().replace(/\r?\n|\r/g, "");

      return cleanToken || null;
    } catch (error) {
      console.error("❌ Error reading .jwtSession file:", error);
      return null;
    }
  });
};

// Initialize authentication
setupAuth();

// Check if we have authentication
const jwtSessionPath = path.join(__dirname, ".jwtSession");
if (fs.existsSync(jwtSessionPath)) {
  try {
    const token = fs.readFileSync(jwtSessionPath, "utf-8").trim();
    console.log("🔐 JWT authentication loaded from .jwtSession file");

    // Try to decode and display basic info (without verification)
    try {
      const payload = JSON.parse(
        Buffer.from(token.split(".")[1], "base64url").toString(),
      );
      console.log(
        `👤 Authenticated as: ${payload.email || payload.sub || "Unknown"}`,
      );

      if (payload.exp) {
        const expiresAt = new Date(payload.exp * 1000);
        const isExpired = Date.now() > payload.exp * 1000;
        console.log(
          `⏰ Token ${isExpired ? "expired" : "expires"}: ${expiresAt.toLocaleString()}`,
        );
      }
    } catch {
      console.log("🔑 JWT token loaded (unable to decode payload)");
    }
  } catch (error) {
    console.error("⚠️  Warning: Could not read .jwtSession file:", error);
  }
} else {
  console.log("ℹ️  No .jwtSession file found - running without authentication");
  console.log(
    "💡 Create a .jwtSession file with your JWT token to enable authentication",
  );
}

// Load existing history
const history = loadHistory();

// Helper function to start REPL with a delay
const startREPL = async () => {
  // Add a small delay to ensure all preamble messages are displayed
  await new Promise(resolve => setTimeout(resolve, 100));
  
  const r = repl.start({
    prompt: "convex> ",
    useColors: true,
    useGlobal: true,
  });
  
  return r;
};

// Main async function to setup REPL
const main = async () => {
  // Start REPL with delay
  const r = await startREPL();

  // Apply loaded history to the REPL
  if (history.length > 0) {
    // Add history to the readline interface
    (r as any).history = history.slice().reverse(); // REPL expects history in reverse order
    console.log(`📚 Loaded ${history.length} commands from history`);
  }

  r.context.client = client;
  r.context.api = api;

  // Add clearHistory function to context
  r.context.clearHistory = () => {
    clearHistory();
    // Also clear the current REPL session history
    (r as any).history = [];
  };

  // Add subscription functions to context
  r.context.subscribe = (queryFn: any, args: any = {}, callback?: (data: any) => void) => {
  const subscriptionId = ++subscriptionCounter;
  
  // Get function name for display - handle Convex function references
  let queryName = 'unknown';
  try {
    // Try different ways to extract the function name
    if (queryFn && typeof queryFn === 'object') {
      // Check for various potential name properties
      if (queryFn._name) {
        queryName = String(queryFn._name);
      } else if (queryFn.name) {
        queryName = String(queryFn.name);
      } else if (queryFn.functionName) {
        queryName = String(queryFn.functionName);
      } else {
        // Fallback: try to extract from the object structure
        queryName = 'query';
      }
    }
  } catch (error) {
    queryName = 'query';
  }
  
  // Use provided callback or default one
  const finalCallback = callback || defaultSubscriptionCallback(subscriptionId, queryName);
  
  try {
    console.log(`🔔 Starting subscription [${subscriptionId}] to ${queryName}...`);
    
    // Check if client has onUpdate method, otherwise use polling fallback
    if (typeof client.onUpdate === 'function') {
      // Subscribe using client.onUpdate (React client)
      const unsubscribe = client.onUpdate(queryFn, args, finalCallback);
      
      // Store the unsubscribe function
      subscriptions.set(subscriptionId, unsubscribe);
    } else {
      // Fallback: polling-based subscription
      console.log(`⚠️  Using polling fallback (onUpdate not available)`);
      
      let lastResult: any = undefined;
      let isRunning = true;
      
      const pollQuery = async () => {
        try {
          if (!isRunning) return;
          
          const result = await client.query(queryFn, args);
          
          // Only call callback if result changed
          if (JSON.stringify(result) !== JSON.stringify(lastResult)) {
            lastResult = result;
            finalCallback(result);
          }
        } catch (error) {
          console.error(`❌ Subscription [${subscriptionId}] polling error:`, error);
        }
        
        // Schedule next poll
        if (isRunning) {
          setTimeout(pollQuery, 2000); // Poll every 2 seconds
        }
      };
      
      // Start polling
      pollQuery();
      
      // Store the unsubscribe function
      const unsubscribe = () => {
        isRunning = false;
        console.log(`🔕 Stopped polling for subscription [${subscriptionId}]`);
      };
      
      subscriptions.set(subscriptionId, unsubscribe);
    }
    
    console.log(`✅ Subscription [${subscriptionId}] active. Use unsubscribe(${subscriptionId}) to stop.`);
    
    return subscriptionId;
  } catch (error) {
    console.error(`❌ Subscription [${subscriptionId}] failed:`, error);
    throw error;
  }
};

  r.context.unsubscribe = (subscriptionId: number) => {
  const unsubscribe = subscriptions.get(subscriptionId);
  if (unsubscribe) {
    unsubscribe();
    subscriptions.delete(subscriptionId);
    console.log(`🔕 Unsubscribed from subscription [${subscriptionId}]`);
  } else {
    console.log(`⚠️  Subscription [${subscriptionId}] not found`);
  }
};

  r.context.listSubscriptions = () => {
  if (subscriptions.size === 0) {
    console.log("📭 No active subscriptions");
    return;
  }
  
  console.log(`📡 Active subscriptions (${subscriptions.size}):`);
  for (const [id] of subscriptions) {
    console.log(`   [${id}] Active subscription`);
  }
};

  r.context.unsubscribeAll = () => {
  const count = subscriptions.size;
  
  for (const [id, unsubscribe] of subscriptions) {
    unsubscribe();
  }
  
  subscriptions.clear();
  
  if (count > 0) {
    console.log(`🔕 Unsubscribed from ${count} subscription${count !== 1 ? 's' : ''}`);
  } else {
    console.log("📭 No subscriptions to unsubscribe from");
  }
};

  // Add list function to context
  r.context.list = async (filter?: string) => {
  try {
    const functions = await getConvexFunctions();
    formatFunctionList(functions, filter);
  } catch (error) {
    console.error('❌ Error listing functions:', error);
    console.log('💡 Try: Object.keys(api) to see available modules');
  }
};

  // Add refreshAuth function to context
  r.context.refreshAuth = () => {
  setupAuth();

  const jwtSessionPath = path.join(__dirname, ".jwtSession");
  if (fs.existsSync(jwtSessionPath)) {
    try {
      const token = fs.readFileSync(jwtSessionPath, "utf-8").trim();
      console.log("🔐 JWT authentication refreshed from .jwtSession file");

      // Try to decode and display basic info (without verification)
      try {
        const payload = JSON.parse(
          Buffer.from(token.split(".")[1], "base64url").toString(),
        );
        console.log(
          `👤 Authenticated as: ${payload.email || payload.sub || "Unknown"}`,
        );

        if (payload.exp) {
          const expiresAt = new Date(payload.exp * 1000);
          const isExpired = Date.now() > payload.exp * 1000;
          console.log(
            `⏰ Token ${isExpired ? "expired" : "expires"}: ${expiresAt.toLocaleString()}`,
          );
        }
      } catch {
        console.log("🔑 JWT token loaded (unable to decode payload)");
      }
    } catch (error) {
      console.error("⚠️  Warning: Could not read .jwtSession file:", error);
    }
  } else {
    console.log("ℹ️  No .jwtSession file found - cleared authentication");
  }
};

  r.context.query = async (fn: any, args?: any) => {
  try {
    return await client.query(fn, args || {});
  } catch (error) {
    console.error("Query error:", error);
    throw error;
  }
};

  r.context.mutation = async (fn: any, args?: any) => {
  try {
    return await client.mutation(fn, args || {});
  } catch (error) {
    console.error("Mutation error:", error);
    throw error;
  }
};

  r.context.action = async (fn: any, args?: any) => {
  try {
    return await client.action(fn, args || {});
  } catch (error) {
    console.error("Action error:", error);
    throw error;
  }
};

  r.context.help = () => {
  console.log(`
🔧 Available globals:
- client: ConvexReactClient instance
- api: Generated API object with your functions
- query(fn, args): Helper to run queries
- mutation(fn, args): Helper to run mutations  
- action(fn, args): Helper to run actions
- subscribe(fn, args, callback?): Subscribe to query with real-time updates
- unsubscribe(id): Stop a specific subscription
- listSubscriptions(): Show all active subscriptions
- unsubscribeAll(): Stop all subscriptions
- list(filter?): Show all available Convex functions
- refreshAuth(): Reload authentication from .jwtSession file
- clearHistory(): Clear command history

🔐 JWT Authentication:
- Create a .jwtSession file in the root directory with your JWT token
- The REPL will automatically load and use this token for authentication
- Use refreshAuth() to reload the token if you update the file

📚 Available functions:
- api.auth.checkIdentity (test JWT authentication)

📖 Command History:
- Use ↑/↓ arrow keys to navigate command history
- History is automatically saved to .convex_repl_history
- Use clearHistory() to clear saved history

📡 Real-time Subscriptions:
- Subscribe to queries for live updates as data changes
- Use subscribe() with default formatting or custom callbacks
- Manage subscriptions with unsubscribe() and listSubscriptions()

💡 Examples:
  // Discover available functions
  list()                  // Show all functions organized by type
  list('queries')         // Show only queries
  list('auth')            // Show functions in auth module
  list('get')             // Show functions containing 'get'
  
  // Test authentication
  action(api.auth.checkIdentity, {})
  
  // Subscribe to a query with default formatting
  const sub1 = subscribe(api.auth.queryIdentity, {})
  
  // Subscribe with custom callback
  const sub2 = subscribe(api.auth.queryIdentity, {}, (data) => {
    console.log("🔔 Identity changed:", data.ident?.email || "Not authenticated")
  })
  
  // Manage subscriptions
  listSubscriptions()     // Show active subscriptions
  unsubscribe(sub1)      // Stop specific subscription
  unsubscribeAll()       // Stop all subscriptions
  
  // Generate a JWT token (in terminal):
  // npm run create-jwt -- --email="user@example.com" --userId="user123" --raw > .jwtSession

🏃‍♂️ Pro tip: All functions return promises, so use 'await' or '.then()'
  `);
};

  console.log(
    "✅ REPL ready! Type 'help()' for available commands and examples.",
  );
  console.log("🎯 Your Convex functions are available via the 'api' object.");

  // Save history on exit
  r.on("exit", () => {
  console.log("\n💾 Saving command history...");

  // Get current REPL history and save it
  const currentHistory = (r as any).history;
  if (currentHistory && currentHistory.length > 0) {
    // REPL history is in reverse order, so reverse it back to chronological order
    const chronologicalHistory = currentHistory.slice().reverse();
    saveHistory(chronologicalHistory);
    console.log(`📚 Saved ${chronologicalHistory.length} commands to history`);
  }

  // Clean up subscriptions
  if (subscriptions.size > 0) {
    console.log(`🔕 Cleaning up ${subscriptions.size} active subscription${subscriptions.size !== 1 ? 's' : ''}...`);
    for (const [id, unsubscribe] of subscriptions) {
      unsubscribe();
    }
    subscriptions.clear();
  }

    console.log("👋 Goodbye!");
    process.exit(0);
  });

  // Also save history on process termination signals
  const handleExit = () => {
    const currentHistory = (r as any).history;
    if (currentHistory && currentHistory.length > 0) {
      const chronologicalHistory = currentHistory.slice().reverse();
      saveHistory(chronologicalHistory);
    }
    
    // Clean up subscriptions
    for (const [id, unsubscribe] of subscriptions) {
      unsubscribe();
    }
    subscriptions.clear();
  };

  process.on("SIGINT", handleExit);
  process.on("SIGTERM", handleExit);
  process.on("beforeExit", handleExit);
};

// Call main function
main().catch(console.error);
