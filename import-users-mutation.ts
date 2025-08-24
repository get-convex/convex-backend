import { mutation } from "./_generated/server";
import { v } from "convex/values";

export const importBulkUsers = mutation({
  args: {
    users: v.array(
      v.object({
        clerkUserId: v.string(),
        email: v.string(),
        emailVerified: v.boolean(),
        name: v.string(),
        firstName: v.optional(v.string()),
        lastName: v.optional(v.string()),
        imageUrl: v.optional(v.string()),
        isActive: v.boolean(),
        joinDate: v.number(),
        role: v.union(v.literal("admin"), v.literal("manager"), v.literal("employee")),
        department: v.optional(v.string()),
        tokenIdentifier: v.string(),
      })
    ),
  },
  handler: async (ctx, args) => {
    const results = [];
    
    for (const user of args.users) {
      try {
        // Check if user already exists by clerkUserId
        const existing = await ctx.db
          .query("users")
          .withIndex("by_clerk_user_id", (q) => q.eq("clerkUserId", user.clerkUserId))
          .first();
        
        if (existing) {
          results.push({
            clerkUserId: user.clerkUserId,
            email: user.email,
            status: "skipped",
            message: "User already exists",
            existingId: existing._id,
          });
          continue;
        }
        
        // Insert new user
        const userId = await ctx.db.insert("users", {
          clerkUserId: user.clerkUserId,
          email: user.email,
          emailVerified: user.emailVerified,
          name: user.name,
          firstName: user.firstName,
          lastName: user.lastName,
          imageUrl: user.imageUrl || "",
          isActive: user.isActive,
          joinDate: user.joinDate,
          role: user.role,
          department: user.department,
          tokenIdentifier: user.tokenIdentifier,
        });
        
        results.push({
          clerkUserId: user.clerkUserId,
          email: user.email,
          status: "created",
          userId,
        });
      } catch (error) {
        results.push({
          clerkUserId: user.clerkUserId,
          email: user.email,
          status: "error",
          error: error instanceof Error ? error.message : "Unknown error",
        });
      }
    }
    
    const summary = {
      total: args.users.length,
      created: results.filter(r => r.status === "created").length,
      skipped: results.filter(r => r.status === "skipped").length,
      errors: results.filter(r => r.status === "error").length,
    };
    
    return {
      summary,
      results,
    };
  },
});