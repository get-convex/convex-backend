const CONVEX_URL = "http://localhost:8000";

// These JSDoc type annotations help VS Code find types.
/** @type {import("convex/browser")["ConvexClient"]} */
const ConvexClient = convex.ConvexClient;
const client = new ConvexClient(CONVEX_URL);

/** @type {import("./convex/_generated/api")["api"]} */
const api = convex.anyApi;

client.onUpdate(api.messages.list, {}, (messages) => {
  console.log(messages);
  const container = document.querySelector(".messages");
  container.innerHTML = "";
  for (const message of messages.reverse()) {
    const li = document.createElement("li");
    li.textContent = `${message.author}: ${message.body}`;
    container.appendChild(li);
  }
});

document.querySelector("form").addEventListener("submit", (e) => {
  e.preventDefault();
  const inp = e.target.querySelector("input");
  client.mutation(api.messages.send, {
    body: inp.value,
    author: "me",
  });
  inp.value = "";
});
