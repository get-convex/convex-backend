import { httpAction } from "./_generated/server";

export default httpAction(async (_, request) => {
  const url = new URL(request.url);
  return new Response(url.href, {
    headers: {
      "content-type": "text/plain",
    },
    status: 200,
  });
});
