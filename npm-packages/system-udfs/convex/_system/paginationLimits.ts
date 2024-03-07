// The maximum number of rows read in a single transaction is higher than this number,
// but we are conservative with the max number of rows read in pagination because we may
// read other documents earlier when deciding which filters to apply.
export const maximumRowsRead = 10000;

// The maximum number of bytes read in a single transaction is higher than this number,
// but we are conservative with the max number of bytes read in pagination because we may
// read other documents earlier when deciding which filters to apply.
export const maximumBytesRead = 5000000;
