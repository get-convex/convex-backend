import { useEffect, useMemo, useRef, useState } from "react";
import { Id } from "../convex/_generated/dataModel";
import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { LatencyBuffer } from "./plot";

export function useStateBoolFromUrl(key: string) {
  const initial = useMemo(() => {
    return new URLSearchParams(window.location.search).has(key);
  }, [key]);
  return useState(initial);
}

export function useStateIntFromUrl(key: string, defaultValue: number) {
  const initial = useMemo(() => {
    const value = parseInt(
      new URLSearchParams(window.location.search).get(key) as string,
      10,
    );
    return isNaN(value) ? defaultValue : value;
  }, [key, defaultValue]);
  return useState(initial);
}

// Change the innerHTML of elements with data-time properties
export function useUpdatingTimeAgo() {
  useEffect(() => {
    let running = true;
    function updateTime() {
      const now = performance.now();
      for (const el of document.querySelectorAll(
        `[data-time]`,
      ) as NodeListOf<HTMLSpanElement>) {
        const ago = Math.round(now - parseFloat(el.dataset.time!));
        if (ago > 999) {
          const agoByTenthsSecond = Math.round(ago / 100) / 10;
          el.innerHTML = agoByTenthsSecond + " s ago";
        } else {
          const agoBy10ms = Math.round(ago / 10) * 10;
          el.innerHTML = ("00" + agoBy10ms).slice(-3) + " ms ago";
        }
      }
      if (running) requestAnimationFrame(updateTime);
    }
    updateTime();
    return function cleanup() {
      running = false;
    };
  }, []);
}

export function useClientState() {
  const sessionId = useRef("");
  if (!sessionId.current) {
    sessionId.current = "session " + Math.round(Math.random() * 1000);
  }
  const latencyBuffer = useRef<LatencyBuffer>();
  if (!latencyBuffer.current) {
    latencyBuffer.current = new LatencyBuffer();
  }

  return {
    sessionId: sessionId.current!,
    latencyBuffer: latencyBuffer.current!,
  };
}

export function usePositions(latencyBuffer: LatencyBuffer, sessionId: string) {
  const [sent, setSent] = useState({ sentNum: 0, sentValue: 0 });
  const [received, setReceived] = useState({ receivedNum: 0, receivedTime: 0 });

  const memoizedEmptyArr = useMemo(() => [], []);
  const positions = useQuery(api.getPositions.default, {}) ?? memoizedEmptyArr;
  const positionsRef = useRef(positions);

  // attempt to detect updates
  if (positions.length !== 0 && positionsRef.current !== positions) {
    const localNow = performance.now();
    const now = Date.now();
    for (const pos of positions) {
      const latency = now - pos.clientSentTs;
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const _latency2 = now - pos.serverSentTs;
      latencyBuffer.record(latency, pos.session, localNow);
    }
    // hack to set state in render
    setTimeout(
      () =>
        setReceived(({ receivedNum }) => ({
          receivedNum: receivedNum + 1,
          receivedTime: localNow,
        })),
      0,
    );
  }

  // use refs to avoid resetting the timer loop
  const reportPosition = [
    useMutation(api.reportPosition.report),
    useMutation(api.reportPosition.reportContentiously),
  ][~~window.contentious];
  const reportPositionRef = useRef(reportPosition);
  reportPositionRef.current = reportPosition;

  const dbId = useRef<Id | null>(null);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    async function report() {
      const [x, y, ts] = [
        Math.floor(Math.random() * 10),
        Math.floor(Math.random() * 10),
        Date.now(),
      ];
      setSent(({ sentNum }) => ({ sentNum: sentNum + 1, sentValue: x }));

      // Note `report()` is scheduled before awaiting the mutation to avoid
      // backpressure. Backpressure is a good idea! But we're testing without
      // it here.
      timer = setTimeout(report, Math.round(1000 / window.qps));
      dbId.current = await reportPositionRef.current({
        x,
        y,
        ts,
        session: sessionId,
        id: dbId.current,
      });
    }
    report();
    return () => clearTimeout(timer);
  }, [sessionId]);

  positionsRef.current = positions;
  const ret = useMemo(
    () => ({ positions, ...received, ...sent }),
    [positions, received, sent],
  );
  return ret;
}
