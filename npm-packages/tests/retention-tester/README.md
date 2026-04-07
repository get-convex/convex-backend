## About

retention-tester is a retention torture tester. It inserts fake data into two
tables every N seconds. It then cleans up old data every M minutes. See the
crons.ts file for how frequently these are run.

To turn this off: Ensure there is a single row in the `yield` table with
`doYouYield` set to `true`. You can do this by running the `yield:convexYields`
function.
