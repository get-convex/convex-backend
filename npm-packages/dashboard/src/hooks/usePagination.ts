import { useState } from "react";

export function usePagination<T>({
  items,
  itemsPerPage,
}: {
  items: T[];
  itemsPerPage: number;
}) {
  const totalPages = Math.ceil(items.length / itemsPerPage);

  const [currentPageUnsafe, setCurrentPage] = useState(1);
  const currentPage = Math.max(1, Math.min(currentPageUnsafe, totalPages));

  const startIndex = (currentPage - 1) * itemsPerPage;
  const endIndex = startIndex + itemsPerPage;

  const visibleItems = items.slice(startIndex, endIndex);

  return {
    visibleItems,
    totalPages,
    currentPage,
    setCurrentPage,
  };
}
