import { Card } from "./components/Card";
import Snowflake from "./components/Snowflake";
import NodeFetch from "./components/NodeFetch";
import Tiktoken from "./components/Tiktoken";

function App() {
  return (
    <div className="w-screen h-fit flex flex-row flex-wrap p-10 gap-10">
      <Card
        className="flex flex-col justify-center items-center gap-4 font-bold text-lg"
        title={"snowflake-sdk"}
        modalContent={<Snowflake />}
      >
        snowflake-sdk
      </Card>
      <Card
        className="flex flex-col justify-center items-center gap-4 font-bold text-lg"
        title={"node-fetch"}
        modalContent={<NodeFetch />}
      >
        node-fetch
      </Card>
      <Card
        className="flex flex-col justify-center items-center gap-4 font-bold text-lg"
        title={"tiktoken"}
        modalContent={<Tiktoken />}
      >
        tiktoken
      </Card>
      <Card
        className="flex flex-col justify-center items-center gap-4 font-bold text-lg"
        title={"ffmpeg-wasm"}
        modalContent={<Snowflake />}
      >
        ffmpeg-wasm
      </Card>
      <Card
        className="flex flex-col justify-center items-center gap-4 font-bold text-lg"
        title={"stockfish"}
        modalContent={<Snowflake />}
      >
        stockfish
      </Card>
    </div>
  );
}

export default App;
