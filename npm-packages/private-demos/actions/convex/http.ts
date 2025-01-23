import { httpRouter } from "convex/server";
import { api } from "./_generated/api";
import { httpAction } from "./_generated/server";

const GIPHY_KEY = "QrXTp0FioARhBHalPs2tpA4RNOTLhFYs";

function giphyUrl(queryString: string) {
  return (
    "https://api.giphy.com/v1/gifs/translate?api_key=" +
    GIPHY_KEY +
    "&s=" +
    encodeURIComponent(queryString)
  );
}

const postGifMessage = httpAction(async ({ runMutation }, request) => {
  const { author, body } = await request.json();

  const data = await fetch(giphyUrl(body));
  const json = await data.json();
  if (!data.ok) {
    throw new Error(`Giphy errored: ${JSON.stringify(json)}`);
  }
  const gifEmbedUrl = json.data.embed_url;

  // Write GIF url to Convex.
  await runMutation(api.sendMessage.default, {
    format: "giphy",
    body: gifEmbedUrl,
    author,
  });
  return new Response(null, {
    status: 200,
  });
});

const http = httpRouter();

http.route({
  path: "/postGifMessage",
  method: "POST",
  handler: postGifMessage,
});

export default http;
