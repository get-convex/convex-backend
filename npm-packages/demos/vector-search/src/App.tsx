import { useState } from "react";
import Food from "./Food";
import Movies from "./Movies";

export default function App() {
  const [selectedPage, setSelectedPage] = useState<"food" | "movies">("food");

  return (
    <main>
      <button onClick={() => setSelectedPage("food")}>Search foods</button>
      <button onClick={() => setSelectedPage("movies")}>Search movies</button>
      {selectedPage === "food" ? <Food /> : <Movies />}
    </main>
  );
}
