export type ChartData = {
  data: Array<object>;
  xAxisKey: string;
  lineKeys: Array<{ key: string; name: string; color: string }>;
};

export type ChartDataSource = (start: Date, end: Date) => Promise<ChartData>;
