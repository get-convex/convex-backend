const colorSchemes = [
  [
    "hsl(255, 60%, 36%)",
    "hsl(37, 35.7%, 55.5%)",
    "hsl(346, 100%, 85%)",
    "hsl(42, 97%, 54%)",
  ],
  [
    "hsl(3, 100%, 32%)",
    "hsl(42, 100%, 80%)",
    "hsl(29, 89%, 54%)",
    "hsl(0, 0%, 36%)",
  ],
  [
    "hsl(270, 13%, 27%)",
    "hsl(220, 56%, 78%)",
    "hsl(316, 59%, 77%)",
    "hsl(260, 60%, 51%)",
  ],
  [
    "hsl(220, 14%, 45%)",
    "hsl(120, 22%, 62%)",
    "hsl(6, 100%, 74%)",
    "hsl(312, 33%, 71%)",
  ],
  [
    "hsl(220, 14%, 45%)",
    "hsl(262, 87%, 74%)",
    "hsl(240, 70%, 42%)",
    "hsl(210, 66%, 84%)",
  ],
  [
    "hsl(6, 100%, 74%)",
    "hsl(40, 80%, 75%)",
    "hsl(316, 59%, 65%)",
    "hsl(42, 100%, 80%)",
  ],
];

function hashString(str: string) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash);
}

export function Avatar({
  name = "",
  hashKey: slug = "",
  size = "medium",
}: {
  name?: string;
  hashKey?: string;
  size?: "medium" | "large";
}) {
  const initial = (
    name.split(" ").length > 1
      ? name.split(" ")[0][0] + name.split(" ")[1][0]
      : name.slice(0, 2)
  ).replace(/[^a-zA-Z0-9]/g, "");
  const hash = hashString(slug);
  const patternIdx = hash % 4;
  const baseColors = colorSchemes[Math.floor(hash / 4) % colorSchemes.length];
  const rotationDeg = hash % 360;
  let gradient: string;
  switch (patternIdx) {
    case 0:
      gradient = `linear-gradient(${rotationDeg}deg, ${baseColors[0]}, ${baseColors[1]})`;
      break;
    case 1:
      gradient = `linear-gradient(${45 + rotationDeg}deg, ${baseColors[2]}, ${baseColors[3]})`;
      break;
    case 2:
      gradient = `linear-gradient(${135 + rotationDeg}deg, ${baseColors[2]}, ${baseColors[0]} 60%, ${baseColors[1]})`;
      break;
    default: {
      const theta = (rotationDeg / 180) * Math.PI;
      const x = 50 + 30 * Math.cos(theta);
      const y = 50 + 30 * Math.sin(theta);
      gradient = `radial-gradient(circle at ${x}% ${y}%, ${baseColors[0]} 0%, ${baseColors[1]} 80%, ${baseColors[2]} 100%)`;
    }
  }
  return (
    <span
      className={`relative inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full select-none ${
        size === "large" ? "size-12" : "size-7"
      }`}
      style={{ backgroundImage: gradient, backgroundSize: "cover" }}
    >
      <span
        aria-hidden
        className="pointer-events-none absolute inset-0 z-10 rounded-full bg-black/30 dark:bg-black/15"
      />
      <span
        className="relative z-20 text-sm leading-none font-medium text-white"
        style={{ textShadow: "0 0 3px rgba(0, 0, 0, 0.5)" }}
      >
        {initial}
      </span>
    </span>
  );
}
