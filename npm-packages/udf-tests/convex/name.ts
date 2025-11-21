import { query } from "./_generated/server";

function f() {
  return 1;
}

export default query(f);

export const g = query({
  args: {},
  handler: function g() {
    return 2;
  },
});

export const h = query({
  args: {},
  handler: () => 3,
});

export const i = function () {
  return 4;
};
