import "./App.css";
import { useQuery } from "convex/react";
import { api } from "../convex/_generated/api";

function App() {
  const tasks = useQuery({
    query: api.tasks.get,
    args: {},
    throwOnError: true,
  }).data;
  return (
    <div className="App">
      {tasks?.map(({ _id, text }) => <div key={_id}>{text}</div>)}
    </div>
  );
}

export default App;
