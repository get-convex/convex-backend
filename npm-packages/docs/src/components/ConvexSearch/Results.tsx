import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import { DotsHorizontalIcon } from "@radix-ui/react-icons";
import algoliasearch from "algoliasearch/lite";
import React, { useEffect, useState } from "react";
import ResultList from "./ResultList";
import { AlgoliaResponse, AlgoliaResult, KapaResponse, Result } from "./types";

// Search-only API key, safe to use in frontend code. See:
// https://www.algolia.com/doc/guides/security/api-keys/#search-only-api-key
const searchClient = algoliasearch(
  "1KIE511890",
  "07096f4c927e372785f8453f177afb16",
);

interface ResultsProps {
  query: string;
}

export default function Results({ query }: ResultsProps) {
  const [algoliaResults, setAlgoliaResults] = useState<Result[]>([]);
  const [kapaResults, setKapaResults] = useState<Result[]>([]);
  const [loading, setLoading] = useState(false);
  const { siteConfig } = useDocusaurusContext();

  const combinedResults = [...algoliaResults, ...kapaResults].reduce<Result[]>(
    (acc, result) => {
      if (
        result.title &&
        !acc.some((existingResult) => existingResult.url === result.url)
      ) {
        acc.push(result);
      }
      return acc;
    },
    [],
  );

  useEffect(() => {
    if (query) {
      // Clear existing results.
      setAlgoliaResults([]);
      setKapaResults([]);

      // Show the loading indicator.
      setLoading(true);

      const algoliaSearch = searchClient
        .search<AlgoliaResult>([
          {
            indexName: "docs",
            query,
            params: {
              hitsPerPage: 3,
            },
          },
        ])
        .then((response) => {
          const hits = (response.results as AlgoliaResponse)[0].hits;
          setAlgoliaResults(
            hits.map((hit) => ({
              // Unclear why, but some pages are indexed with an empty title.
              title: hit.title === "" ? "Convex Docs" : hit.title,
              url: hit.objectID,
              snippet: hit.contents,
            })),
          );
        });

      // Search Kapa
      const kapaSearch = fetch(
        `https://api.kapa.ai/query/v1/projects/${siteConfig.customFields.KAPA_AI_PROJECT}/search/`,
        {
          method: "POST",
          headers: {
            "X-API-KEY": siteConfig.customFields.KAPA_AI_KEY as string,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            query: query,
          }),
        },
      )
        .then((response) => response.json())
        .then((data: KapaResponse) => {
          setKapaResults(
            data.search_results.map((hit) => {
              // The API returns titles in the format "page|heading", where
              // heading may be blank.
              let [page, heading] = hit.title.split("|");

              // Remove prefix before Discord results.
              page = page.replace("Discord support thread: ", "");

              // Clear heading if it matches the page exactly.
              if (heading === page) {
                heading = "";
              }

              return {
                title: heading !== "" ? heading : page,
                url: hit.source_url,
                ...(heading !== "" && { subtext: page }),
              };
            }),
          );
        });

      Promise.all([algoliaSearch, kapaSearch]).finally(() => {
        setLoading(false);
      });
    }
  }, [query, siteConfig]);

  return (
    <div className="flex flex-col overflow-hidden gap-3 grow">
      <div
        className="overflow-y-auto grow flex flex-col gap-2"
        // Keeps the scroll bar light in dark mode.
        style={{
          colorScheme: "light",
        }}
      >
        <ResultList results={combinedResults} />
        {loading && (
          <div className="flex justify-center items-center">
            <DotsHorizontalIcon className="w-8 h-8 text-neutral-n8 animate-pulse" />
          </div>
        )}
      </div>
    </div>
  );
}
