// @snippet start httpAction
import { ActionCtx, httpAction, mutation, query } from "./_generated/server";
import { api } from "./_generated/api";

export const postMessage = httpAction(async (ctx, request) => {
  const { author, body } = await request.json();

  await ctx.runMutation(api.messages.send, {
    body: `Sent via HTTP action: ${body}`,
    author,
  });

  return new Response(null, {
    status: 200,
  });
});
// @snippet end httpAction

export const list = query({
  handler: async (ctx) => {
    return await ctx.db.query("messages").collect();
  },
});

export const send = mutation({
  handler: async (ctx, { body, author }) => {
    const message = { body, author };
    await ctx.db.insert("messages", message);
  },
});

const queryByAuthor = async (ctx: ActionCtx, authorNumber: string) => {
  const messages = await ctx.runQuery(api.messages.list);
  const filteredMessages = messages
    .filter((message) => {
      return message.author === `User ${authorNumber}`;
    })
    .map((message) => {
      return {
        body: message.body,
        author: message.author,
      };
    });
  return new Response(JSON.stringify(filteredMessages), {
    headers: {
      "content-type": "application/json",
    },
    status: 200,
  });
};

export const getByAuthor = httpAction(async (ctx, request) => {
  const url = new URL(request.url);
  const authorNumber =
    url.searchParams.get("authorNumber") ??
    request.headers.get("authorNumber") ??
    null;
  if (authorNumber === null) {
    return new Response(
      "Did not specify authorNumber as query param or header",
      {
        status: 400,
      },
    );
  }
  return await queryByAuthor(ctx, authorNumber);
});

export const getByAuthorPathSuffix = httpAction(async (ctx, request) => {
  const url = new URL(request.url);
  const pathParts = url.pathname.split("/");
  if (pathParts.length < 3) {
    return new Response(
      "Missing authorNumber path suffix, URL path should be in the form /getAuthorMessages/[author]",
    );
  }
  const authorNumber = pathParts[pathParts.length - 1];
  return await queryByAuthor(ctx, authorNumber);
});
