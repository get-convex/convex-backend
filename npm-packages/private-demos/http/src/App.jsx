import { useRef, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

export default function App() {
  const messages = useQuery(api.listMessages.default) || [];

  const [newMessageText, setNewMessageText] = useState("");
  const sendMessage = useMutation(api.sendMessage.default);

  const imageInput = useRef(null);
  const [selectedImage, setSelectedImage] = useState(null);

  const [name] = useState(() => "User " + Math.floor(Math.random() * 10000));
  async function handleSendMessage(event) {
    event.preventDefault();
    setNewMessageText("");
    if (newMessageText) {
      await sendMessage({ body: newMessageText, author: name });
    }
  }

  async function handleSendImageWithHttp(event) {
    event.preventDefault();
    setSelectedImage(null);
    imageInput.current.value = "";

    const sendImageUrl = new URL("http://127.0.0.1:8001/sendImage");
    sendImageUrl.searchParams.set("author", name);

    const digest = await crypto.subtle.digest(
      "SHA-256",
      await selectedImage.arrayBuffer(),
    );
    const base64Digest = btoa(String.fromCharCode(...new Uint8Array(digest)));
    const digestHeader = `sha-256=${base64Digest}`;
    await fetch(sendImageUrl, {
      method: "POST",
      headers: { "Content-Type": selectedImage.type, digest: digestHeader },
      body: selectedImage,
    });
    return;
  }

  return (
    <main>
      <h1>Convex Chat</h1>
      <p className="badge">
        <span>{name}</span>
      </p>
      <ul>
        {messages.map((message) => (
          <li key={message._id}>
            <span>{message.author}:</span>
            {message.format === "image" ? (
              <>
                <ImageWithHttpRedirect storageId={message.body} />
                <ImageWithHttp storageId={message.body} />
                <DeleteImage storageId={message.body} />
              </>
            ) : (
              <span>{message.body}</span>
            )}
            <span>{new Date(message._creationTime).toLocaleTimeString()}</span>
          </li>
        ))}
      </ul>
      <form onSubmit={handleSendMessage}>
        <input
          value={newMessageText}
          onChange={(event) => setNewMessageText(event.target.value)}
          placeholder="Write a message…"
        />
        <input type="submit" value="Send" disabled={!newMessageText} />
      </form>
      <form onSubmit={handleSendImageWithHttp}>
        <input
          type="file"
          accept="image/*"
          ref={imageInput}
          onChange={(event) => setSelectedImage(event.target.files[0])}
          disabled={selectedImage}
        />
        <input type="submit" value="Send Image" disabled={!selectedImage} />
      </form>
    </main>
  );
}

function ImageWithHttp({ storageId }) {
  const getImageUrl = new URL("http://127.0.0.1:8001/getImage");
  getImageUrl.searchParams.set("storageId", storageId);

  return (
    <div>
      <div>Image via HTTP</div>
      <img src={getImageUrl.href} height="300px" width="auto" />
    </div>
  );
}

function ImageWithHttpRedirect({ storageId }) {
  const getImageUrl = new URL("http://127.0.0.1:8001/getImageWithRedirect");
  getImageUrl.searchParams.set("storageId", storageId);
  return (
    <div>
      <div>Image via HTTP Redirect</div>
      <img src={getImageUrl.href} height="300px" width="auto" />
    </div>
  );
}

function DeleteImage({ storageId }) {
  const deleteImageUrl = new URL("http://127.0.0.1:8001/deleteImage");
  deleteImageUrl.searchParams.set("storageId", storageId);

  return (
    <div onClick={async () => await fetch(deleteImageUrl, { method: "POST" })}>
      X
    </div>
  );
}
