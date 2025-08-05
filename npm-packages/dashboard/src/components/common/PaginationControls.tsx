import {
  ChevronLeftIcon,
  ChevronRightIcon,
  DoubleArrowLeftIcon,
  DoubleArrowRightIcon,
} from "@radix-ui/react-icons";
import { Button } from "@ui/Button";

interface PaginationControlsProps {
  currentPage: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  className?: string;
}

export function PaginationControls({
  currentPage,
  totalPages,
  onPageChange,
  className = "",
}: PaginationControlsProps) {
  if (totalPages <= 1) {
    return null;
  }

  return (
    <div className={`flex items-center justify-center gap-2 ${className}`}>
      {/* First page button */}
      <Button
        variant="neutral"
        inline
        size="sm"
        icon={<DoubleArrowLeftIcon />}
        onClick={() => onPageChange(1)}
        disabled={currentPage === 1}
      />

      {/* Previous page button */}
      <Button
        variant="neutral"
        inline
        size="sm"
        icon={<ChevronLeftIcon />}
        onClick={() => onPageChange(Math.max(1, currentPage - 1))}
        disabled={currentPage === 1}
      />

      {/* Page indicator */}
      <span className="text-sm text-content-secondary">
        Page {currentPage} of {totalPages}
      </span>

      {/* Next page button */}
      <Button
        variant="neutral"
        inline
        size="sm"
        icon={<ChevronRightIcon />}
        onClick={() => onPageChange(Math.min(totalPages, currentPage + 1))}
        disabled={currentPage === totalPages}
      />

      {/* Last page button */}
      <Button
        variant="neutral"
        inline
        size="sm"
        icon={<DoubleArrowRightIcon />}
        onClick={() => onPageChange(totalPages)}
        disabled={currentPage === totalPages}
      />
    </div>
  );
}
