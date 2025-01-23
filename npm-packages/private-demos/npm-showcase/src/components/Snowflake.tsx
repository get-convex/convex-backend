import TextInput from "./TextInput";
import { useState } from "react";
import { useAction } from "convex/react";
import { api } from "../../convex/_generated/api";

export function Snowflake() {
  const [queryValue, setQueryValue] = useState("");
  const [loadingQuery, setLoadingQuery] = useState(false);
  const [result, setResult] = useState("");
  const doSqlQuery = useAction(api.snowflake.doSqlQuery);

  const handleSubmit = async () => {
    setResult("");
    setLoadingQuery(true);
    try {
      setResult(await doSqlQuery({ stmt: queryValue }));
    } finally {
      setLoadingQuery(false);
    }
  };

  return (
    <div className={"flex flex-col gap-4 items-start"}>
      <div>SQL Query</div>
      <TextInput
        value={queryValue}
        onChange={(ev) => setQueryValue(ev.target.value)}
        placeholder={"Query"}
        disabled={loadingQuery}
      />
      <button
        type={"submit"}
        className={
          "rounded-lg border p-3 text-white hover:bg-blue-800 bg-blue-500 transition"
        }
        disabled={loadingQuery}
        onClick={handleSubmit}
      >
        {loadingQuery ? "Loading" : "Submit"}
      </button>
      <div className={"text-green-500 w-full break-words"}>{result}</div>
    </div>
  );
}

export default Snowflake;
