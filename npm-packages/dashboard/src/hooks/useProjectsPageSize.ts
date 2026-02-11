import { useGlobalLocalStorage } from "@common/lib/useGlobalLocalStorage";

export function useProjectsPageSize() {
  const [projectsPageSize, setProjectsPageSize] = useGlobalLocalStorage(
    "projectsPageSize",
    24,
  );
  return { pageSize: projectsPageSize, setPageSize: setProjectsPageSize };
}

export const PROJECT_PAGE_SIZES = [
  { label: "6", value: 6 },
  { label: "12", value: 12 },
  { label: "24", value: 24 },
  { label: "48", value: 48 },
  { label: "96", value: 96 },
];
