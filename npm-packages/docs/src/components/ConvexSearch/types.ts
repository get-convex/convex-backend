export type AlgoliaResult = {
  title: string;
  objectID: string;
  contents: string;
};

export type AlgoliaResponse = {
  hits: AlgoliaResult[];
}[];

export type KapaResult = {
  title: string;
  source_type: string;
  source_url: string;
  content: string;
};

export type KapaResponse = {
  search_results: KapaResult[];
};

export type Result = {
  title: string;
  url: string;
  subtext?: string;
};
