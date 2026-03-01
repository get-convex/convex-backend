import { query, mutation, action } from "./_generated/server";

// Test setting log attributes in a query
export const queryWithAttributes = query({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({
      user_id: "user_123",
      operation: "test_query",
      count: 42,
    });
    return "query completed";
  },
});

// Test setting log attributes in a mutation
export const mutationWithAttributes = mutation({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({
      user_id: "user_456",
      action_type: "create",
      success: true,
    });
    return "mutation completed";
  },
});

// Test setting log attributes in an action
export const actionWithAttributes = action({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({
      external_api: "test_service",
      request_id: "req_789",
      latency_ms: 150,
    });
    return "action completed";
  },
});

// Test multiple setLogAttributes calls (should merge)
export const multipleSetCalls = mutation({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({
      first_key: "first_value",
      shared_key: "original",
    });
    ctx.setLogAttributes({
      second_key: "second_value",
      shared_key: "overwritten",
    });
    return "merged attributes";
  },
});

// Test with various value types
export const variousTypes = query({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({
      string_val: "hello",
      number_val: 123.456,
      bool_true: true,
      bool_false: false,
      int_val: 42,
    });
    return "various types set";
  },
});

// Test empty attributes (should be valid)
export const emptyAttributes = query({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({});
    return "empty attributes";
  },
});

// Test OTel-style dot-separated keys
export const dotSeparatedKeys = query({
  args: {},
  handler: async (ctx) => {
    ctx.setLogAttributes({
      "http.method": "POST",
      "http.status_code": 200,
      "user.id": "user_123",
      "service.name": "my_service",
    });
    return "dot-separated keys set";
  },
});
