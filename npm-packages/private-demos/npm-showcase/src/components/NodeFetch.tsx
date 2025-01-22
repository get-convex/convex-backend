import { useState } from "react";
import { useAction } from "convex/react";
import { api } from "../../convex/_generated/api";

export function NodeFetch() {
  const [urlValue, setUrlValue] = useState("");
  const [loadingFetch, setLoadingFetch] = useState(false);
  const [result, setResult] = useState("");
  const doFetch = useAction(api.node_fetch.fetchUrl);

  const handleSubmit = async () => {
    setResult("");
    setLoadingFetch(true);
    try {
      setResult(await doFetch({ url: urlValue }));
    } finally {
      setLoadingFetch(false);
    }
  };

  return (
    <div className={"flex flex-col gap-4 items-start"}>
      <div>Url to GET</div>
      <input
        value={urlValue}
        onChange={(ev) => setUrlValue(ev.target.value)}
        placeholder={"URL"}
        disabled={loadingFetch}
        className={"w-full border rounded-sm p-1"}
      />
      <button
        type={"submit"}
        className={
          "rounded-lg border p-3 text-white hover:bg-blue-800 bg-blue-500 transition"
        }
        disabled={loadingFetch}
        onClick={handleSubmit}
      >
        {loadingFetch ? "Loading" : "Submit"}
      </button>
      <div className={"text-green-500 w-full break-words"}>{result}</div>
    </div>
  );
}

export default NodeFetch;
