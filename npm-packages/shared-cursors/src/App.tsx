import { ReactElement, useCallback, useEffect, useRef } from "react";
import { ConvexProvider, ConvexReactClient } from "convex/react";

import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import {
  useClientState,
  usePositions,
  useStateBoolFromUrl,
  useStateIntFromUrl,
  useUpdatingTimeAgo,
} from "./hooks";
import { COLORS, Plot } from "./plot";

const address: string = import.meta.env.VITE_CONVEX_URL;

declare global {
  // Global variables that live outside of React to prevent rerenders. These
  // could be passed around as refs or stuck in a context that shared refs,
  // but global variables are real convenient.
  interface Window {
    qps: number;
    contentious: boolean;
  }
}

export default function App() {
  const [numClients, setNumClients] = useStateIntFromUrl("clients", 1);
  const [numSubscriptions, setNumSubscriptions] = useStateIntFromUrl(
    "subscriptions-per-client",
    1,
  );
  useUpdatingTimeAgo();

  return (
    <main style={{ display: "flex", flexDirection: "column" }}>
      <h1>Are we multiplayer yet?</h1>
      <QPSSlider />
      <span>{`${numSubscriptions} ${
        numSubscriptions === 1 ? "subscription" : "subscriptions"
      } per client`}</span>
      <SubscriptionsPerClientSlider
        onChange={setNumSubscriptions}
        initial={numSubscriptions}
      />
      <span>{`${numClients} ${numClients === 1 ? "client" : "clients"}`}</span>
      <ClientsSlider onChange={setNumClients} initial={numClients} />
      <ContentionCheckbox />
      <button
        style={{ maxWidth: 250 }}
        onClick={() => {
          const origNumClients = numClients;
          setNumClients(0);
          setTimeout(() => setNumClients(origNumClients), 100);
        }}
      >
        Clear
      </button>
      {Array.from({ length: numClients })
        .map((_, i) => i)
        .map((i) => (
          <WrappedClient
            key={i}
            first={i === 0}
            i={i}
            color={COLORS[i]}
            subscriptions={numSubscriptions}
          />
        ))}
    </main>
  );
}

function ExtraSubscriptions({
  num,
  clientNum,
}: {
  num: number;
  clientNum: number;
}) {
  return (
    <>
      {Array.from({ length: num }).map((_, i) => (
        <ExtraSubscription key={i} nonce={`client${clientNum}_sub${i + 2}`} />
      ))}
    </>
  );
}

function ExtraSubscription({ nonce }: { nonce: string }) {
  useQuery(api.getPositions.default, { nonce });
  return null;
}

type ClientProps = {
  first: boolean;
  i: number;
  color: string;
  subscriptions: number;
};

function WrappedClient(props: ClientProps) {
  const client = useRef<ConvexReactClient>();
  if (!client.current) client.current = new ConvexReactClient(address);
  useEffect(() => {
    return () => {
      client.current && client.current.close();
      client.current = undefined;
    };
  }, []);
  return (
    <ConvexProvider client={client.current!}>
      <Client {...props} />
      <ExtraSubscriptions num={props.subscriptions - 1} clientNum={props.i} />
    </ConvexProvider>
  );
}

function Client({ first, color }: ClientProps) {
  const { sessionId, latencyBuffer } = useClientState();
  const { receivedNum, receivedTime, sentNum } = usePositions(
    latencyBuffer,
    sessionId,
  );
  const clear = useMutation(api.clear.default);

  // Every time a first client on the page is created, clear the db.
  useEffect(() => {
    if (first) clear();
  }, [first, clear]);

  const height = 100;

  return (
    <>
      <div style={{ display: "flex" }}>
        <div
          className={sentNum % 2 === 0 ? "just-sent1" : "just-sent2"}
          style={{
            width: 100,
            height,
            ...CENTER_STYLES,
            color: "white",
            border: `solid 10px ${color}`,
          }}
        >
          sent
        </div>
        <div
          className={
            receivedNum % 2 === 0 ? "just-received1" : "just-received2"
          }
          style={{ width: 100, height, ...CENTER_STYLES, color: "white" }}
        >
          received update
          <br />
          <span
            style={{ fontVariantNumeric: "tabular-nums" }}
            data-time={receivedTime}
          >
            000 ms ago
          </span>
        </div>
        <Plot buffer={latencyBuffer} width={300} height={height} />
      </div>
    </>
  );
}

const CENTER_STYLES = {
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  flexDirection: "column",
  textAlign: "center",
  lineHeight: "1em",
} as const;

function QPSSlider(): ReactElement {
  const [initial] = useStateIntFromUrl("qps", 1);
  const onChange = useCallback((n: number) => {
    window.qps = n;
    document.querySelector("#timeoutValue")!.innerHTML = display(window.qps);
  }, []);
  if (window.qps === undefined) {
    window.qps = initial;
  }

  return (
    <>
      <span id="timeoutValue">{display(window.qps)}</span>
      <input
        type="range"
        style={{ maxWidth: 400 }}
        defaultValue={initial}
        min="1"
        max={30}
        onInput={(e: any) => onChange(parseInt(e.target.value))}
      ></input>
    </>
  );
}

function ClientsSlider({
  onChange,
  initial,
}: {
  onChange: (n: number) => void;
  initial: number;
}): ReactElement {
  return (
    <input
      type="range"
      style={{ maxWidth: 200 }}
      defaultValue={initial}
      min={0}
      max={8}
      onInput={(e: any) => onChange(parseInt(e.target.value))}
    ></input>
  );
}

function SubscriptionsPerClientSlider({
  onChange,
  initial,
}: {
  onChange: (n: number) => void;
  initial: number;
}): ReactElement {
  return (
    <input
      type="range"
      style={{ maxWidth: 200 }}
      defaultValue={initial}
      min={1}
      max={20}
      onInput={(e: any) => onChange(parseInt(e.target.value))}
    ></input>
  );
}

function ContentionCheckbox(): ReactElement {
  const [initial] = useStateBoolFromUrl("contentious");
  if (window.contentious === undefined) {
    window.contentious = initial;
  }
  const onChange = useCallback((checked: boolean) => {
    window.contentious = checked;
  }, []);
  return (
    <div>
      <label>
        <input
          style={{ marginRight: 4 }}
          type="checkbox"
          defaultChecked={initial}
          onChange={(e) => onChange(e.target.checked)}
        />
        extra contention: find doc to modify via filter instead of db.get()
      </label>
    </div>
  );
}

function display(qps: number): string {
  return `sending updates at ${qps} QPS, every ${Math.round(1000 / qps)}ms`;
}
