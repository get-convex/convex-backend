function newDirection(
  { colIndex, rowIndex }: { colIndex: number; rowIndex: number },
  key: string,
) {
  switch (key) {
    case "ArrowLeft":
      return { newColIndex: colIndex - 1, newRowIndex: rowIndex };
    case "ArrowRight":
      return { newColIndex: colIndex + 1, newRowIndex: rowIndex };
    case "ArrowUp":
      return { newColIndex: colIndex, newRowIndex: rowIndex - 1 };
    case "ArrowDown":
      return { newColIndex: colIndex, newRowIndex: rowIndex + 1 };
    default:
      return { newColIndex: colIndex, newRowIndex: rowIndex };
  }
}

export const arrowKeyHandler =
  (cellRef: React.MutableRefObject<HTMLDivElement | HTMLLabelElement | null>) =>
  (event: React.KeyboardEvent<HTMLButtonElement | HTMLInputElement>) => {
    if (event.key.startsWith("Arrow")) {
      const wrapper = cellRef.current?.parentElement;
      const row = wrapper?.parentElement;
      const rows = row?.parentElement;
      const colIndex = Array.prototype.indexOf.call(row?.childNodes, wrapper);
      const rowIndex = Array.prototype.indexOf.call(rows?.childNodes, row);

      const { newColIndex, newRowIndex } = newDirection(
        { colIndex, rowIndex },
        event.key,
      );
      const newTarget = rows?.children[newRowIndex]?.children[newColIndex]
        ?.firstElementChild as HTMLElement;
      (newTarget?.firstElementChild as HTMLButtonElement)?.focus();
    }
  };
