module.exports = {
  plain: {
    color: "hsl(220, 14%, 71%)",
    backgroundColor: "hsl(220, 13%, 18%)",
  },
  styles: [
    {
      types: ["comment", "prolog"],
      style: {
        color: "hsl(220, 10%, 40%)",
      },
    },
    {
      types: ["doctype", "punctuation", "entity"],
      style: {
        color: "hsl(220, 14%, 71%)",
      },
    },
    { types: ["class-name"], style: { color: "#e5c07b" } },
    {
      types: ["attr-name", "boolean", "constant", "number", "atrule"],
      style: {
        color: "hsl(29, 54%, 61%)",
      },
    },
    {
      types: ["keyword"],
      style: {
        color: "hsl(286, 60%, 67%)",
      },
    },
    {
      types: ["property", "tag", "symbol", "deleted", "important"],
      style: {
        color: "hsl(355, 65%, 65%)",
      },
    },
    {
      types: [
        "selector",
        "string",
        "char",
        "builtin",
        "inserted",
        "regex",
        "attr",
      ],
      style: {
        color: "hsl(95, 38%, 62%)",
      },
    },
    {
      types: ["variable", "operator", "function"],
      style: {
        color: "hsl(207, 82%, 66%)",
      },
    },
    {
      types: ["url"],
      style: {
        color: "hsl(187, 47%, 55%)",
      },
    },
  ],
};
