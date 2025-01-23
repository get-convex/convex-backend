// @snippet start httpFileUpload
import { useRef, useState } from "react";

const convexSiteUrl = import.meta.env.VITE_CONVEX_SITE_URL;

export default function App() {
  const imageInput = useRef(null);
  const [selectedImage, setSelectedImage] = useState(null);

  async function handleSendImage(event) {
    event.preventDefault();

    // e.g. https://happy-animal-123.convex.site/sendImage?author=User+123
    const sendImageUrl = new URL(`${convexSiteUrl}/sendImage`);
    sendImageUrl.searchParams.set("author", "Jack Smith");

    await fetch(sendImageUrl, {
      method: "POST",
      headers: { "Content-Type": selectedImage.type },
      body: selectedImage,
    });

    setSelectedImage(null);
    imageInput.current.value = "";
  }
  return (
    <form onSubmit={handleSendImage}>
      <input
        type="file"
        accept="image/*"
        ref={imageInput}
        onChange={(event) => setSelectedImage(event.target.files[0])}
        disabled={selectedImage !== null}
      />
      <input
        type="submit"
        value="Send Image"
        disabled={selectedImage === null}
      />
    </form>
  );
}
// @snippet end httpFileUpload
