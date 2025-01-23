// Functions are still internal.
import { ConvexHttpClient } from "convex/browser";
import { useState, FormEvent } from "react";
import { useAction, useMutation, useQuery } from "convex/react";
import { Doc } from "../convex/_generated/dataModel";
import { api } from "../convex/_generated/api";

const randomName = "User " + Math.floor(Math.random() * 10000);

// Render a chat message.
function MessageView(props: { message: Doc<"messages"> }) {
  const message = props.message;
  if (message.format === "giphy") {
    return (
      <div>
        <div>
          <strong>{message.author}:</strong>
        </div>
        <iframe src={message.body} />
        <div className="giphy-attribution">Powered By GIPHY</div>
      </div>
    );
  }
  return (
    <div>
      <strong>{message.author}:</strong> {message.body}
    </div>
  );
}

function ChatBox() {
  // Dynamically update `messages` in response to the output of
  // `listMessages.ts`.
  const messages = useQuery(api.listMessages.default) || [];
  return (
    <div className="chat-box">
      <ul className="list-group shadow-sm my-3">
        {messages.map((message: any) => (
          <li
            key={message._id}
            className="list-group-item d-flex justify-content-between"
          >
            <MessageView message={message} />
            <div className="ml-auto text-secondary text-nowrap">
              {new Date(message._creationTime).toLocaleTimeString()}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Commands() {
  const convexHttpClient = new ConvexHttpClient(
    import.meta.env.VITE_CONVEX_URL,
  );
  const sendMessage = useMutation(api.sendMessage.default);
  const sendGiphy = useAction(api.sendGifMessage.default);
  const sayHello = useAction(api.simple.hello);
  //const tiktoken = useAction(api.tiktoken.default);
  const makeSillyError = useAction(api.simple.userError);
  const thisTakesForever = useAction(api.simple.userTimeout);
  const sendEmail = useAction(api.email.default);
  const ticTacToe = useMutation(api.tic.default);
  const langchain = useAction(api.langchain.default);

  const danglingFetch = useAction(api.dangle.danglingFetch);

  // Run `sendMessage.ts` as a mutation to record a chat message when
  // `handleSendMessage` triggered.
  const [newMessageText, setNewMessageText] = useState("");
  async function handleSendMessage(event: FormEvent) {
    event.preventDefault();
    setNewMessageText(""); // reset text entry box

    // If a /giphy command is entered call into the Vercel function to post
    // relevant GIF to channel.
    if (newMessageText.startsWith("/")) {
      if (newMessageText.startsWith("/giphy ")) {
        await sendGiphy({
          queryString: newMessageText.slice(7),
          author: randomName,
        });
      } else if (newMessageText.startsWith("/hello ")) {
        const response = await sayHello({ somebody: newMessageText.slice(7) });
        await sendMessage({
          format: "text",
          body: response,
          author: randomName,
        });
      } else if (newMessageText.startsWith("/http_hello ")) {
        const response = await convexHttpClient.action(api.simple.hello, {
          somebody: newMessageText.slice(12),
        });
        await sendMessage({
          format: "text",
          body: response,
          author: randomName,
        });
      } else if (newMessageText.startsWith("/tic-tac-toe")) {
        await ticTacToe({ author: randomName });
      } else if (newMessageText.startsWith("/gpt4")) {
        const response = await langchain({ prompt: newMessageText.slice(6) });
        await sendMessage({
          format: "text",
          body: response,
          author: randomName,
        });
      } else if (newMessageText.startsWith("/oops")) {
        await makeSillyError();
      } else if (newMessageText.startsWith("/slow")) {
        console.log(await thisTakesForever());
      } else if (newMessageText.startsWith("/email ")) {
        const email = newMessageText.slice(7);
        if (email.endsWith("@convex.dev")) {
          console.log(await sendEmail({ email }));
        } else {
          alert("Must be a @convex.dev email you silly!");
        }
      } else if (newMessageText.startsWith("/danglingFetch")) {
        console.log(await danglingFetch({ url: newMessageText.slice(15) }));
        // } else if (newMessageText.startsWith("/tiktoken ")) {
        //   const response = await tiktoken({ text: newMessageText.slice(10) });
        //   await sendMessage({
        //     format: "text",
        //     body: response,
        //     author: randomName,
        //   });
      } else {
        alert("Invalid command!");
      }
    } else {
      await sendMessage({
        format: "text",
        body: newMessageText,
        author: randomName,
      });
    }
  }

  return (
    <div className="command-box">
      <form
        onSubmit={handleSendMessage}
        className="d-flex justify-content-center"
      >
        <input
          value={newMessageText}
          onChange={(event) => setNewMessageText(event.target.value)}
          className="form-control w-50"
          placeholder="Write a message…"
        />
        <input
          type="submit"
          value="Send"
          disabled={!newMessageText}
          className="ms-2 btn btn-primary"
        />
      </form>
      <div className="command-hints">
        Secret Messages:
        <ul>
          <li>/hello &lt;name&gt; - hi from Hawaii</li>
          <li>/http_hello &lt;name&gt; - hello via http client</li>
          <li>/giphy &lt;query&gt; - posts a giphy (currently quite slow)</li>
          <li>
            /tic-tac-toe - mutation schedules action, which schedules a mutation
          </li>
          <li>
            /gpt4 &lt;question&gt; - use langchain to call OpenAI's "gpt-4" LLM
          </li>
          <li>/email &lt;user&gt;@convex.dev - sends a high email from Tom</li>
          <li>/oops - throws an error (look at browser console)</li>
          <li>/tiktoken &lt;message&gt; - tokenize the message.</li>
          <li>
            /slow - takes an hour (look at browser console after 20s seconds)
          </li>
          <li>/danglingFetch https://example.com</li>
        </ul>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <main className="py-4">
      <h1 className="text-center">Convex Actions Demo</h1>
      <p className="text-center">
        <span className="badge bg-dark">{randomName}</span>
      </p>

      <div className="main-content">
        <ChatBox />
        <Commands />
      </div>
    </main>
  );
}
