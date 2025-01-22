"use node";
import { api } from "./_generated/api";
import { action } from "./_generated/server";

// Replace this with your own GIPHY key obtained at
// https://developers.giphy.com/ -> Create Account.
const GIPHY_KEY = "QrXTp0FioARhBHalPs2tpA4RNOTLhFYs";

function giphyUrl(queryString: string) {
  return (
    "https://api.giphy.com/v1/gifs/translate?api_key=" +
    GIPHY_KEY +
    "&s=" +
    encodeURIComponent(queryString)
  );
}

// Post a GIF chat message corresponding to the query string.
export default action(
  async (
    { runMutation },
    { queryString, author }: { queryString: string; author: string },
  ) => {
    // Fetch GIF url from GIPHY.
    const data = await fetch(giphyUrl(queryString));
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
  },
);
