import { useEffect, useLayoutEffect, useState } from "react";
import uPlot, { AlignedData, Options } from "uplot";

// D3's Tableau10
export const COLORS = (
  "4e79a7f28e2ce1575976b7b259a14fedc949af7aa1ff9da79c755fbab0ab".match(
    /.{6}/g,
  ) as string[]
).map((x) => `#${x}`);

const TIME_WINDOW_MS = 11000;
export class LatencyBuffer {
  ys: Record<string, (number | null)[]> = {};
  xs: number[] = [];
  constructor() {
    this.clear();
  }
  clear() {
    this.ys = {};
    this.xs = [];
  }
  record(latency: number, client: string, localNow: number) {
    if (!(client in this.ys)) {
      // TypeScript is wrong, `fill()` accepts null
      this.ys[client] = Array.from(this.xs).fill(null as unknown as number);
    }

    this.xs.push(localNow);
    for (const c of Object.keys(this.ys)) {
      this.ys[c].push(null);
    }
    this.ys[client][this.ys[client].length - 1] = latency;
  }
  clearOlderThanWindow(now: number) {
    const tenSecondsAgo = now - TIME_WINDOW_MS;
    while (this.xs.length && this.xs[0] < tenSecondsAgo) this.xs.shift();
    for (const sess of Object.keys(this.ys)) {
      const arr = this.ys[sess];
      while (arr.length > this.xs.length) arr.shift();
    }
  }
  getData(): AlignedData {
    const localNow = performance.now();
    this.clearOlderThanWindow(localNow);
    const datas = Object.values(this.ys);
    const now = Date.now();
    const pageStartTimeS = now / 1000 - localNow / 1000;
    const xs = this.xs.map((x: number) => pageStartTimeS + x / 1000);
    return [xs, ...datas];
  }
}

type PlotProps = {
  buffer: LatencyBuffer;
  width: number;
  height: number;
};

export function Plot({ buffer, width, height }: PlotProps) {
  const [el, setEl] = useState<HTMLDivElement | null>(null);
  const [plot, setPlot] = useState<uPlot>();

  useLayoutEffect(() => {
    if (!el) return;
    const opts: Options = {
      width,
      height,
      series: [{}],
      scales: {
        y: { range: [10, 10000], distr: 3 },
      },
      axes: [
        {
          show: false,
          side: 0,
        },
        {
          ticks: { size: 0 },
          side: 1,
        },
      ],
      legend: {
        show: false,
      },
    };
    const data: AlignedData = [[], []];
    setPlot(new uPlot(opts, data, el));
  }, [el, width, height]);

  useEffect(() => {
    let reqId: ReturnType<typeof requestAnimationFrame> = 0;
    function update() {
      if (plot && buffer) {
        const data = buffer.getData();
        plot.setData(data);
        while (plot.series.length < data.length) {
          plot.addSeries(
            {
              stroke: COLORS[plot.series.length - 1],
              spanGaps: true,
              pxAlign: 0,
              points: { show: false },
            },
            plot.series.length,
          );
        }
        while (plot.series.length > data.length) {
          plot.delSeries(plot.series.length - 1);
        }
        const now = Date.now() / 1000;
        const tenSecondsAgo = now - 10;
        plot.setScale("x", { min: tenSecondsAgo, max: now });
      }
      reqId = requestAnimationFrame(update);
    }
    update();
    return function cleanup() {
      cancelAnimationFrame(reqId);
    };
  }, [plot, buffer]);

  return <div ref={setEl} style={{ width, height }}></div>;
}
