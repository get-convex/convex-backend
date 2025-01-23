export function toggleAdjacent<I>(
  rows: I[],
  rowIndex: number,
  isRowSelected: (id: I) => boolean,
  toggleIsRowSelected: (id: I) => void,
) {
  const checked = isRowSelected(rows[rowIndex]);

  if (checked) {
    // Unselect this row and all the next consecutive selected
    for (let i = rowIndex; i < rows.length; i++) {
      const id = rows[i];
      if (!isRowSelected(id)) break;
      toggleIsRowSelected(id);
    }
  } else {
    // If there are no rows selected above, first try to select from below
    const firstSelected = rows.findIndex((r) => isRowSelected(r));
    if (firstSelected > rowIndex) {
      for (let i = rowIndex; i < firstSelected; i++) {
        toggleIsRowSelected(rows[i]);
      }
      return;
    }

    // Select all rows from the first unselected row above
    for (let i = rowIndex; i >= 0; i--) {
      const id = rows[i];
      if (isRowSelected(id)) break;
      toggleIsRowSelected(id);
    }
  }
}
