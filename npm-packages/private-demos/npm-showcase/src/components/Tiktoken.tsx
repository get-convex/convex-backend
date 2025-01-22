import { useState } from "react";
import { useAction } from "convex/react";
import { api } from "../../convex/_generated/api";

export function Tiktoken() {
  const [inputString, setInputString] = useState("");
  const [decodeArr, setDecodeArr] = useState("");
  const [loadingEncode, setLoadingEncode] = useState(false);
  const [loadingDecode, setLoadingDecode] = useState(false);
  const [encodeResult, setEncodeResult] = useState<number[]>([]);
  const [decodeResult, setDecodeResult] = useState("");
  const encode = useAction(api.tiktoken.encode);
  const decode = useAction(api.tiktoken.decode);

  const handleEncode = async () => {
    setLoadingEncode(true);
    try {
      setEncodeResult(await encode({ str: inputString }));
    } finally {
      setLoadingEncode(false);
    }
  };

  const handleDecode = async () => {
    setLoadingDecode(true);
    // parse the array
    const decodeArrNoPrefix =
      decodeArr.length > 0 && decodeArr.charAt(0) === "["
        ? decodeArr.slice(1)
        : decodeArr;
    const decodeArrFinal =
      decodeArr.length > 0 && decodeArr.charAt(decodeArr.length - 1) === "]"
        ? decodeArrNoPrefix.slice(0, -1)
        : decodeArrNoPrefix;
    const arr = decodeArrFinal.split(",").map(Number);

    setDecodeResult(await decode({ arr }));
    setLoadingDecode(false);
  };

  return (
    <div className={"flex flex-col gap-4 items-start"}>
      <div>tiktoken</div>
      <input
        value={inputString}
        onChange={(ev) => setInputString(ev.target.value)}
        placeholder={"string to encode"}
        disabled={loadingEncode}
        className={"w-full border rounded-sm p-1"}
      />
      <button
        type={"submit"}
        className={
          "rounded-lg border p-3 text-white hover:bg-blue-800 bg-blue-500 transition"
        }
        disabled={loadingEncode}
        onClick={handleEncode}
      >
        {loadingEncode ? "Loading" : "Submit"}
      </button>
      <div className={"text-green-500 w-full break-words"}>
        {JSON.stringify(encodeResult)}
      </div>
      <input
        value={decodeArr}
        onChange={(ev) => setDecodeArr(ev.target.value)}
        placeholder={"array to decode"}
        disabled={loadingDecode}
        className={"w-full border rounded-sm p-1"}
      />
      <button
        type={"submit"}
        className={
          "rounded-lg border p-3 text-white hover:bg-blue-800 bg-blue-500 transition"
        }
        disabled={loadingDecode}
        onClick={handleDecode}
      >
        {loadingDecode ? "Loading" : "Submit"}
      </button>
      <div className={"text-green-500 w-full break-words"}>{decodeResult}</div>
    </div>
  );
}

export default Tiktoken;
