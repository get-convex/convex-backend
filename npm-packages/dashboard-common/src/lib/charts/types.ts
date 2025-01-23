export type ChartData = {
  data: Array<Object>;
  xAxisKey: string;
  lineKeys: Array<{ key: string; name: string; color: string }>;
};

export type ChartDataSource = (start: Date, end: Date) => Promise<ChartData>;
