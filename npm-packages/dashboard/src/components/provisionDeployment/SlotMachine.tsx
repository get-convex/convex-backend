import { cn } from "@ui/cn";
import { Spinner } from "@ui/Spinner";
import { useCurrentTheme } from "@common/lib/useCurrentTheme";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// Convex brand colors — sourced from the design-system CSS variables
const BRAND = {
  purple: "var(--purple-500)",
  // eslint-disable-next-line no-restricted-syntax
  red: "var(--red-500)",
  yellow: "var(--yellow-500)",
  purpleGlow: "color-mix(in srgb, var(--purple-500) 50%, transparent)",
  // eslint-disable-next-line no-restricted-syntax
  redGlow: "color-mix(in srgb, var(--red-500) 40%, transparent)",
  yellowGlow: "color-mix(in srgb, var(--yellow-500) 50%, transparent)",
} as const;

// Emoji mappings for animals
const ANIMAL_EMOJI: Record<string, string> = {
  aardvark: "🐽",
  akita: "🐕",
  albatross: "🦅",
  alligator: "🐊",
  alpaca: "🦙",
  anaconda: "🐍",
  ant: "🐜",
  anteater: "🐽",
  antelope: "🦌",
  armadillo: "🦔",
  avocet: "🪶",
  axolotl: "🦎",
  badger: "🦡",
  bandicoot: "🐀",
  barracuda: "🐟",
  basilisk: "🦎",
  bass: "🐟",
  bat: "🦇",
  beagle: "🐶",
  bear: "🐻",
  bee: "🐝",
  bird: "🐦",
  bison: "🦬",
  blackbird: "🐦‍⬛",
  bloodhound: "🐕",
  boar: "🐗",
  bobcat: "🐱",
  buffalo: "🐃",
  bulldog: "🐶",
  bullfrog: "🐸",
  butterfly: "🦋",
  buzzard: "🦅",
  caiman: "🐊",
  camel: "🐪",
  canary: "🐤",
  capybara: "🦫",
  cardinal: "🐦",
  caribou: "🦌",
  cassowary: "🐦",
  cat: "🐱",
  caterpillar: "🐛",
  chameleon: "🦎",
  cheetah: "🐆",
  chickadee: "🐦",
  chicken: "🐔",
  chihuahua: "🐕",
  chinchilla: "🐹",
  chipmunk: "🐿️",
  civet: "🐾",
  clam: "🐚",
  clownfish: "🐠",
  cobra: "🐍",
  cod: "🐟",
  condor: "🦅",
  corgi: "🐕",
  cormorant: "🐦",
  cow: "🐄",
  coyote: "🐺",
  crab: "🦀",
  crane: "🦩",
  cricket: "🦗",
  crocodile: "🐊",
  crow: "🐦‍⬛",
  curlew: "🪶",
  cuttlefish: "🦑",
  dachshund: "🐕",
  dalmatian: "🐕",
  deer: "🦌",
  dinosaur: "🦕",
  dodo: "🦤",
  dog: "🐶",
  dogfish: "🐟",
  dolphin: "🐬",
  donkey: "🫏",
  dotterel: "🐦",
  dove: "🕊️",
  dragon: "🐉",
  duck: "🦆",
  eagle: "🦅",
  echidna: "🦤",
  eel: "🐍",
  egret: "🪶",
  elephant: "🐘",
  elk: "🦌",
  emu: "🐦",
  ermine: "🐾",
  falcon: "🦅",
  fennec: "🦊",
  ferret: "🦦",
  finch: "🐦",
  firefly: "🪲",
  fish: "🐟",
  flamingo: "🦩",
  fly: "🪰",
  fox: "🦊",
  frog: "🐸",
  gazelle: "🦌",
  gecko: "🦎",
  gerbil: "🐹",
  giraffe: "🦒",
  gnat: "🪰",
  gnu: "🐃",
  goat: "🐐",
  goldfinch: "🐦",
  goldfish: "🐠",
  goose: "🪿",
  gopher: "🐹",
  goshawk: "🦅",
  grasshopper: "🦗",
  greyhound: "🐕",
  grouse: "🐦",
  guanaco: "🦙",
  guineapig: "🐹",
  gull: "🐦",
  hamster: "🐹",
  hare: "🐇",
  hawk: "🦅",
  hedgehog: "🦔",
  heron: "🪶",
  herring: "🐟",
  hippopotamus: "🦛",
  hornet: "🐝",
  horse: "🐎",
  hound: "🐕",
  hummingbird: "🐦",
  husky: "🐺",
  hyena: "🐺",
  ibex: "🐐",
  ibis: "🪶",
  iguana: "🦎",
  impala: "🦌",
  jackal: "🐺",
  jaguar: "🐆",
  jay: "🐦",
  jellyfish: "🪼",
  kangaroo: "🦘",
  kingfisher: "🐦",
  kiwi: "🐦",
  koala: "🐨",
  kookabura: "🐦",
  kudu: "🦌",
  labrador: "🐕",
  ladybug: "🐞",
  lapwing: "🐦",
  lark: "🐦",
  lemming: "🐹",
  lemur: "🐒",
  leopard: "🐆",
  lion: "🦁",
  llama: "🦙",
  lobster: "🦞",
  loris: "🐒",
  lynx: "🐱",
  lyrebird: "🐦",
  magpie: "🐦",
  malamute: "🐕",
  mallard: "🦆",
  mammoth: "🦣",
  manatee: "🐳",
  mandrill: "🐒",
  marlin: "🐟",
  marmot: "🐿️",
  marten: "🐾",
  mastiff: "🐕",
  meadowlark: "🐦",
  meerkat: "🦦",
  mink: "🐾",
  minnow: "🐟",
  mockingbird: "🐦",
  mole: "🐾",
  mongoose: "🐾",
  monitor: "🦎",
  moose: "🫎",
  mosquito: "🦟",
  mouse: "🐭",
  mule: "🫏",
  narwhal: "🐳",
  newt: "🦎",
  nightingale: "🐦",
  ocelot: "🐆",
  octopus: "🐙",
  okapi: "🦒",
  opossum: "🐾",
  orca: "🐳",
  oriole: "🐦",
  ostrich: "🐦",
  otter: "🦦",
  owl: "🦉",
  ox: "🐂",
  oyster: "🐚",
  panda: "🐼",
  panther: "🐈‍⬛",
  parakeet: "🦜",
  parrot: "🦜",
  partridge: "🐦",
  peacock: "🦚",
  peccary: "🐗",
  pelican: "🐦",
  penguin: "🐧",
  perch: "🐟",
  pheasant: "🐦",
  pig: "🐷",
  pigeon: "🐦",
  pika: "🐹",
  platypus: "🦦",
  pony: "🐴",
  poodle: "🐩",
  porcupine: "🦔",
  porpoise: "🐬",
  possum: "🐾",
  ptarmigan: "🐦",
  puffin: "🐧",
  puma: "🐆",
  quail: "🐦",
  rabbit: "🐰",
  raccoon: "🦝",
  ram: "🐏",
  rat: "🐀",
  raven: "🐦‍⬛",
  reindeer: "🦌",
  retriever: "🐕",
  rhinoceros: "🦏",
  roadrunner: "🐦",
  robin: "🐦",
  rook: "🐦‍⬛",
  rooster: "🐓",
  salamander: "🦎",
  salmon: "🐟",
  sandpiper: "🐦",
  sardine: "🐟",
  schnauzer: "🐕",
  scorpion: "🦂",
  seahorse: "🐟",
  seal: "🦭",
  setter: "🐕",
  shark: "🦈",
  sheep: "🐑",
  shepherd: "🐕",
  shrimp: "🦐",
  skunk: "🦨",
  snail: "🐌",
  snake: "🐍",
  sockeye: "🐟",
  spaniel: "🐕",
  sparrow: "🐦",
  spider: "🕷️",
  spoonbill: "🪶",
  squid: "🦑",
  squirrel: "🐿️",
  starfish: "⭐",
  starling: "🐦",
  stingray: "🐟",
  stoat: "🐾",
  stork: "🪶",
  sturgeon: "🐟",
  swan: "🦢",
  swordfish: "🐟",
  tapir: "🐾",
  tern: "🐦",
  terrier: "🐕",
  tiger: "🐅",
  toad: "🐸",
  tortoise: "🐢",
  toucan: "🦜",
  trout: "🐟",
  turtle: "🐢",
  viper: "🐍",
  vole: "🐭",
  vulture: "🦅",
  walrus: "🦭",
  warbler: "🐦",
  warthog: "🐗",
  weasel: "🐾",
  whale: "🐋",
  wildcat: "🐈",
  wildebeest: "🐃",
  wolf: "🐺",
  wolverine: "🦡",
  wombat: "🐻",
  woodpecker: "🐦",
  wren: "🐦",
  yak: "🐂",
  zebra: "🦓",
};

// Emoji mappings for adjectives (expressive/abstract emojis)
const ADJECTIVE_EMOJI: Record<string, string> = {
  abundant: "🌟",
  academic: "🎓",
  accomplished: "🏅",
  accurate: "🎯",
  acoustic: "🎵",
  acrobatic: "🤸",
  adamant: "💪",
  adept: "🧑‍🔧",
  adjoining: "🔗",
  admired: "🌟",
  adorable: "🥰",
  adventurous: "🏔️",
  affable: "😊",
  agile: "⚡",
  agreeable: "👍",
  amiable: "😊",
  amicable: "🤝",
  animated: "🎬",
  ardent: "🔥",
  aromatic: "🌺",
  artful: "🎨",
  astute: "🧐",
  avid: "🔥",
  aware: "👁️",
  basic: "📦",
  beaming: "😁",
  befitting: "👌",
  beloved: "❤️",
  benevolent: "🕊️",
  blessed: "🙏",
  blissful: "😇",
  bold: "🅱️",
  brainy: "🧠",
  brave: "🫡",
  brazen: "⚡",
  bright: "✨",
  brilliant: "💡",
  calculating: "🧮",
  calm: "🧘",
  canny: "🧠",
  capable: "🛠️",
  careful: "🔍",
  cautious: "⚠️",
  ceaseless: "♾️",
  charming: "🌹",
  chatty: "💬",
  cheerful: "🌞",
  cheery: "😊",
  clean: "🧼",
  clear: "💧",
  clever: "🤓",
  colorful: "🌈",
  colorless: "⬜",
  combative: "⚔️",
  compassionate: "💗",
  confident: "😎",
  content: "😌",
  cool: "❄️",
  coordinated: "🔄",
  courteous: "🎩",
  curious: "🔍",
  dapper: "🎩",
  dashing: "🏃",
  dazzling: "🌟",
  deafening: "📢",
  decisive: "✅",
  dependable: "🔒",
  descriptive: "📝",
  determined: "💪",
  different: "🔀",
  diligent: "📚",
  disciplined: "📏",
  doting: "💕",
  dusty: "🌫️",
  dutiful: "📋",
  dynamic: "🚀",
  earnest: "🙏",
  effervescent: "🫧",
  efficient: "⚙️",
  elated: "🎉",
  elegant: "🥇",
  enchanted: "🪄",
  enduring: "🏔️",
  energetic: "⚡",
  energized: "🔋",
  exciting: "🎢",
  expert: "🎓",
  exuberant: "🎉",
  fabulous: "💅",
  famous: "⭐",
  fantastic: "🌟",
  fast: "💨",
  fastidious: "🧹",
  fearless: "🤟",
  festive: "🎊",
  fiery: "🔥",
  fine: "👌",
  first: "🥇",
  fleet: "💨",
  flexible: "🦴",
  flippant: "🙃",
  focused: "🎯",
  formal: "👔",
  fortunate: "🍀",
  friendly: "🤝",
  frugal: "💰",
  gallant: "⚔️",
  giant: "🏔️",
  giddy: "🤭",
  glad: "😃",
  glorious: "🌟",
  good: "👍",
  graceful: "🪷",
  grand: "👑",
  grandiose: "🏛️",
  grateful: "🙏",
  greedy: "💰",
  gregarious: "🤗",
  groovy: "🥾",
  hallowed: "🕯️",
  handsome: "✨",
  hardy: "🪨",
  harmless: "🕊️",
  healthy: "💚",
  hearty: "💚",
  helpful: "🤝",
  hidden: "🫣",
  hip: "🎶",
  honorable: "🎖️",
  hushed: "🤫",
  ideal: "💯",
  impartial: "⚖️",
  impressive: "🏆",
  incredible: "🤩",
  industrious: "🏭",
  insightful: "💡",
  intent: "🎯",
  jovial: "😄",
  joyous: "🥳",
  judicious: "⚖️",
  keen: "👁️",
  kindhearted: "💗",
  kindly: "🌸",
  kindred: "🫂",
  knowing: "🧠",
  laudable: "👏",
  limitless: "♾️",
  little: "🐣",
  lovable: "💖",
  lovely: "🌷",
  loyal: "🐕",
  majestic: "🏰",
  marvelous: "🤩",
  mellow: "🌿",
  merry: "🎊",
  mild: "🌤️",
  modest: "🌾",
  moonlit: "🌙",
  nautical: "⚓",
  neat: "🧹",
  necessary: "📌",
  neighborly: "🏡",
  next: "➡️",
  notable: "🤴",
  oceanic: "🌊",
  optimistic: "🌅",
  opulent: "💎",
  original: "🆕",
  outgoing: "🗣️",
  outstanding: "🏆",
  pastel: "🎨",
  patient: "⏳",
  peaceful: "☮️",
  perceptive: "👁️",
  perfect: "💯",
  pleasant: "🌸",
  polished: "💎",
  polite: "🎩",
  posh: "🎀",
  precious: "💍",
  precise: "📐",
  prestigious: "🏅",
  proficient: "🛠️",
  proper: "👔",
  quaint: "🏡",
  qualified: "📜",
  quick: "⚡",
  quiet: "🤫",
  quirky: "🤪",
  quixotic: "🌠",
  rapid: "🏁",
  rare: "💎",
  reliable: "🔒",
  reminiscent: "🕰️",
  resilient: "🌿",
  resolute: "🪨",
  rightful: "⚖️",
  robust: "💪",
  rosy: "🌹",
  rugged: "🪨",
  savory: "🍲",
  scintillating: "✨",
  scrupulous: "🔍",
  secret: "🤫",
  sensible: "🧠",
  shiny: "✨",
  shocking: "⚡",
  silent: "🤐",
  sincere: "💙",
  sleek: "😎",
  small: "🐜",
  spotted: "🔵",
  standing: "🧍",
  steady: "⚓",
  stoic: "🗿",
  striped: "🦓",
  strong: "💪",
  successful: "🏆",
  superb: "🏆",
  tacit: "🤐",
  tame: "🐑",
  tangible: "🖐️",
  terrific: "🌟",
  third: "🥉",
  tidy: "✨",
  tough: "🪨",
  tremendous: "🌟",
  trustworthy: "🤞",
  uncommon: "💎",
  unique: "🦤",
  upbeat: "🎶",
  usable: "🛠️",
  useful: "🔧",
  utmost: "🔝",
  valiant: "🛡️",
  valuable: "💰",
  veracious: "✅",
  vibrant: "🌈",
  vivid: "🎨",
  wandering: "🧭",
  warmhearted: "🫶",
  wary: "👀",
  watchful: "👀",
  whimsical: "🪄",
  wonderful: "⭐",
  wooden: "🪵",
  woozy: "😵‍💫",
  wry: "😏",
  youthful: "🌱",
  zany: "🤡",
  zealous: "🔥",
};

// Derive word lists from emoji map keys
const ADJECTIVES = Object.keys(ADJECTIVE_EMOJI);
const ANIMALS = Object.keys(ANIMAL_EMOJI);

function getEmoji(word: string, type: "adjective" | "animal"): string {
  if (type === "animal") return ANIMAL_EMOJI[word] || "❓";
  return ADJECTIVE_EMOJI[word] || "❓";
}

function shuffle<T>(arr: T[]): T[] {
  const result = [...arr];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

// Generate numbers once, stable across renders
const NUMBERS = Array.from({ length: 50 }, () =>
  String(Math.floor(Math.random() * 1000)),
).filter((v, i, a) => a.indexOf(v) === i);

const ITEM_HEIGHT_EMOJI = 72;
const ITEM_HEIGHT_TEXT = 40;
const REEL_ITEM_COUNT = 30;
const SPIN_SPEED = 20; // pixels per frame at ~60fps
// Fixed deceleration duration ensures reels stop in order
const DECEL_DURATION_MS = 1500;

// 3D cylinder constants
const CYLINDER_ITEM_ANGLE = 24; // degrees between adjacent items on the drum
const CYLINDER_VISIBLE_ITEMS = 11; // how many items to render (centered on current)

type ReelState = "spinning" | "decelerating" | "stopped";
type ReelType = "adjective" | "animal" | "number";

/**
 * Quadratic ease-out: progress 0->1 maps to 0->1 but decelerating.
 * f(t) = t * (2 - t)
 */
function easeOutQuad(t: number): number {
  return t * (2 - t);
}

function useReel(
  pool: string[],
  finalValue: string | undefined,
  stopDelay: number,
  itemHeight: number,
) {
  const [state, setState] = useState<ReelState>("spinning");
  const [offset, setOffset] = useState(0);
  const offsetRef = useRef(0);
  const animFrameRef = useRef<number>();
  const stateRef = useRef<ReelState>("spinning");
  const stopTriggeredRef = useRef(false);

  // Deceleration: time-based for predictable duration
  const decelStartTimeRef = useRef(0);
  const decelStartOffsetRef = useRef(0);
  const decelTotalDistRef = useRef(0);

  // Build the reel strip once on mount, then rebuild only when finalValue arrives
  const reelItems = useMemo(() => {
    const shuffled = shuffle(pool).slice(0, REEL_ITEM_COUNT);
    if (finalValue) {
      const filtered = shuffled.filter((v) => v !== finalValue);
      // Place the final value near the end so the reel travels through items
      filtered.splice(REEL_ITEM_COUNT - 3, 0, finalValue);
      return filtered.slice(0, REEL_ITEM_COUNT);
    }
    return shuffled;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [finalValue]);

  const finalIndex = finalValue ? reelItems.indexOf(finalValue) : -1;
  const targetOffset = finalIndex >= 0 ? finalIndex * itemHeight : -1;

  const animate = useCallback((now: number) => {
    const currentState = stateRef.current;
    if (currentState === "stopped") return;

    if (currentState === "spinning") {
      offsetRef.current += SPIN_SPEED;
    } else if (currentState === "decelerating") {
      const elapsed = now - decelStartTimeRef.current;
      const t = Math.min(1, elapsed / DECEL_DURATION_MS);
      const easedProgress = easeOutQuad(t);

      offsetRef.current =
        decelStartOffsetRef.current + easedProgress * decelTotalDistRef.current;

      if (t >= 1) {
        offsetRef.current =
          decelStartOffsetRef.current + decelTotalDistRef.current;
        setOffset(offsetRef.current);
        stateRef.current = "stopped";
        setState("stopped");
        return;
      }
    }

    setOffset(offsetRef.current);
    animFrameRef.current = requestAnimationFrame(animate);
  }, []);

  // Start spinning on mount
  useEffect(() => {
    stateRef.current = "spinning";
    setState("spinning");
    offsetRef.current = 0;
    stopTriggeredRef.current = false;
    animFrameRef.current = requestAnimationFrame(animate);
    return () => {
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
    };
  }, [animate]);

  // Trigger deceleration after delay when finalValue is set
  useEffect(() => {
    if (!finalValue || stopTriggeredRef.current) return;
    const timer = setTimeout(() => {
      stopTriggeredRef.current = true;

      // Calculate distance to travel: reach the target + one full revolution
      const maxOffset = reelItems.length * itemHeight;
      const currentPos = offsetRef.current % maxOffset;
      let distanceToTarget = targetOffset - currentPos;
      if (distanceToTarget <= 0) {
        distanceToTarget += maxOffset;
      }
      // Add one full revolution so it visually "winds down"
      const totalDistance = distanceToTarget + maxOffset;

      decelStartOffsetRef.current = offsetRef.current;
      decelTotalDistRef.current = totalDistance;
      decelStartTimeRef.current = performance.now();

      stateRef.current = "decelerating";
      setState("decelerating");
    }, stopDelay);
    return () => clearTimeout(timer);
  }, [finalValue, stopDelay, reelItems.length, targetOffset, itemHeight]);

  return { reelItems, offset, state };
}

function usePrefersReducedMotion() {
  const [prefersReduced, setPrefersReduced] = useState(() => {
    if (typeof window === "undefined") return false;
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  });

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handler = (e: MediaQueryListEvent) => setPrefersReduced(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  return prefersReduced;
}

export function SlotMachine({
  deploymentName,
  className,
  forceReducedMotion,
  showEmoji = true,
  onComplete,
}: {
  /** When set, the slot machine will decelerate and stop at this deployment name. Format: "adjective-animal-number" */
  deploymentName?: string;
  className?: string;
  /** Force the reduced-motion fallback view (useful for testing) */
  forceReducedMotion?: boolean;
  /** Show emoji icons above each word (default: true) */
  showEmoji?: boolean;
  /** Called when all reels have stopped spinning */
  onComplete?: () => void;
}) {
  const prefersReducedMotion = usePrefersReducedMotion();
  const [animationKey, setAnimationKey] = useState(0);
  const [replayMessage, setReplayMessage] = useState<string | null>(null);

  // Reduced motion: show a simple loading/resolved state
  if (forceReducedMotion || prefersReducedMotion) {
    return (
      <div
        className={cn(
          "inline-flex items-center gap-1 font-mono text-sm",
          className,
        )}
      >
        {deploymentName ? (
          <span className="font-semibold text-content-primary">
            {deploymentName}
          </span>
        ) : (
          <Spinner />
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center gap-2">
      <SlotMachineAnimated
        key={animationKey}
        deploymentName={deploymentName}
        className={className}
        showEmoji={showEmoji}
        onReplay={() => {
          if (animationKey === 0) {
            setReplayMessage(
              "Note that this slot machine is deterministic, just like Convex",
            );
          } else if (animationKey >= 1) {
            setReplayMessage(
              "Trying your luck? Seriously, you won't get a different result",
            );
          }
          setAnimationKey((k) => k + 1);
        }}
        onComplete={onComplete}
      />
      {replayMessage && (
        <p className="text-xs text-content-primary italic">{replayMessage}</p>
      )}
    </div>
  );
}

function SlotMachineAnimated({
  deploymentName,
  className,
  showEmoji,
  onReplay,
  onComplete,
}: {
  deploymentName?: string;
  className?: string;
  showEmoji: boolean;
  onReplay: () => void;
  onComplete?: () => void;
}) {
  const currentTheme = useCurrentTheme();
  const isDark = currentTheme === "dark";
  const itemHeight = showEmoji ? ITEM_HEIGHT_EMOJI : ITEM_HEIGHT_TEXT;
  const parts = deploymentName?.split("-");
  const finalAdjective = parts?.[0];
  const finalAnimal = parts?.[1];
  const finalNumber =
    parts && parts.length > 2 ? parts.slice(2).join("-") : undefined;

  const adjReel = useReel(ADJECTIVES, finalAdjective, 600, itemHeight);
  const animalReel = useReel(ANIMALS, finalAnimal, 1400, itemHeight);
  const numberReel = useReel(NUMBERS, finalNumber, 2200, itemHeight);

  const allStopped =
    adjReel.state === "stopped" &&
    animalReel.state === "stopped" &&
    numberReel.state === "stopped";

  // Lever: starts pulled (down). When all reels stop, lever springs back up.
  // Clicking the lever (when up) animates it down, then triggers replay.
  const [leverPulled, setLeverPulled] = useState(true);
  useEffect(() => {
    if (allStopped) {
      setLeverPulled(false);
      onComplete?.();
    }
  }, [allStopped, onComplete]);

  const handleLeverClick = useCallback(() => {
    setLeverPulled(true);
    // Wait for pull-down animation to finish, then remount
    setTimeout(onReplay, 400);
  }, [onReplay]);

  const reels: {
    reel: typeof adjReel;
    type: ReelType;
    finalValue: string | undefined;
  }[] = [
    { reel: adjReel, type: "adjective", finalValue: finalAdjective },
    { reel: animalReel, type: "animal", finalValue: finalAnimal },
    { reel: numberReel, type: "number", finalValue: finalNumber },
  ];

  return (
    <div className={cn("inline-flex items-center gap-0", className)}>
      {/* Machine body */}
      <div
        className="relative rounded-xl p-1"
        style={{
          background: `linear-gradient(180deg, ${BRAND.purple}, #5a1a4a 40%, ${BRAND.purple})`,
          boxShadow: isDark
            ? `0 6px 24px rgba(0,0,0,0.5), 0 0 16px ${BRAND.purpleGlow}`
            : `0 4px 16px rgba(0,0,0,0.2), 0 0 12px ${BRAND.purpleGlow}`,
        }}
      >
        <div
          className="rounded-lg p-1.5"
          style={{
            background: isDark
              ? "linear-gradient(180deg, #2a2a2a 0%, #1a1a1a 40%, #111 100%)"
              : "linear-gradient(180deg, #f0f0f0 0%, #e0e0e0 40%, #d8d8d8 100%)",
          }}
        >
          {/* Reel window area — overflow hidden here clips 3D content at the machine edge */}
          <div
            className="relative flex items-stretch overflow-hidden rounded"
            style={{
              background: isDark ? "#080808" : "#ffffff",
            }}
          >
            {reels.map(({ reel, type, finalValue }, i) => (
              <div key={type} className="flex items-stretch">
                {/* Reel window — no overflow:hidden so CSS 3D context is preserved */}
                <div
                  className="relative"
                  style={{
                    height: itemHeight * 3,
                    width: reelWidth(type, showEmoji),
                    perspective: 350,
                  }}
                >
                  <ReelStrip
                    reel={reel}
                    type={type}
                    finalValue={finalValue}
                    showEmoji={showEmoji}
                    itemHeight={itemHeight}
                    isDark={isDark}
                  />
                  {/* Cylinder depth shadow — dark at top/bottom, clear in center */}
                  <div
                    className="pointer-events-none absolute inset-0 z-10"
                    style={{
                      background:
                        "linear-gradient(180deg, rgba(0,0,0,0.5) 0%, transparent 20%, transparent 80%, rgba(0,0,0,0.5) 100%)",
                    }}
                  />
                </div>
                {/* Reel divider — z-20 to sit above the cylinder depth shadow */}
                {i < reels.length - 1 && (
                  <div
                    className="w-px self-stretch"
                    style={{
                      background: `color-mix(in srgb, ${BRAND.purple} 35%, transparent)`,
                      zIndex: 20,
                    }}
                  />
                )}
              </div>
            ))}

            {/* Payline indicator */}
            <div
              className="pointer-events-none absolute inset-x-0 z-30"
              style={{
                top: "50%",
                transform: "translateY(-50%)",
                height: itemHeight,
              }}
            >
              {/* Left arrow */}
              <div
                className="absolute top-1/2 -left-0.5 -translate-y-1/2"
                style={{
                  width: 0,
                  height: 0,
                  borderTop: "6px solid transparent",
                  borderBottom: "6px solid transparent",
                  borderLeft: `7px solid ${BRAND.yellow}`,
                  filter: `drop-shadow(0 0 4px ${BRAND.yellowGlow})`,
                }}
              />
              {/* Right arrow */}
              <div
                className="absolute top-1/2 -right-0.5 -translate-y-1/2"
                style={{
                  width: 0,
                  height: 0,
                  borderTop: "6px solid transparent",
                  borderBottom: "6px solid transparent",
                  borderRight: `7px solid ${BRAND.yellow}`,
                  filter: `drop-shadow(0 0 4px ${BRAND.yellowGlow})`,
                }}
              />
              {/* Top line */}
              <div
                className="absolute inset-x-0 top-0"
                style={{
                  height: 1,
                  background: `color-mix(in srgb, ${BRAND.yellow} 27%, transparent)`,
                }}
              />
              {/* Bottom line */}
              <div
                className="absolute inset-x-0 bottom-0"
                style={{
                  height: 1,
                  background: `color-mix(in srgb, ${BRAND.yellow} 27%, transparent)`,
                }}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Pull lever */}
      <div
        style={{
          position: "relative",
          marginLeft: -4,
          alignSelf: "stretch",
          width: 28,
          perspective: 300,
          zIndex: 10,
          cursor: allStopped && !leverPulled ? "pointer" : "default",
        }}
        role={allStopped && !leverPulled ? "button" : undefined}
        tabIndex={allStopped && !leverPulled ? 0 : undefined}
        onClick={allStopped && !leverPulled ? handleLeverClick : undefined}
        onKeyDown={
          allStopped && !leverPulled
            ? (e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleLeverClick();
                }
              }
            : undefined
        }
        title={allStopped && !leverPulled ? "Pull to spin again" : undefined}
      >
        {/* Pivot mount (fixed at center of machine side) */}
        <div
          style={{
            position: "absolute",
            top: "50%",
            left: 0,
            width: 14,
            height: 18,
            transform: "translateY(-50%)",
            borderRadius: "0 4px 4px 0",
            background: "linear-gradient(180deg, #666 0%, #444 50%, #555 100%)",
            boxShadow:
              "1px 1px 4px rgba(0,0,0,0.6), inset 0 1px 0 rgba(255,255,255,0.1)",
            zIndex: 2,
          }}
        />
        {/* Pivot bolt */}
        <div
          style={{
            position: "absolute",
            top: "50%",
            left: 5,
            width: 8,
            height: 8,
            transform: "translateY(-50%)",
            borderRadius: "50%",
            background:
              "radial-gradient(circle at 35% 35%, #888, #555 60%, #333)",
            boxShadow: "0 1px 2px rgba(0,0,0,0.5)",
            zIndex: 3,
          }}
        />
        {/* Lever arm + knob: pivots from the mount point via rotateX.
            rotateX(55deg) = arm tilted away (up/ready state).
            rotateX(-55deg) = arm tilted toward viewer (pulled/down state). */}
        <div
          style={{
            position: "absolute",
            top: "50%",
            left: 5,
            width: 8,
            height: 80,
            transformOrigin: "50% 0%",
            transformStyle: "preserve-3d",
            transform: leverPulled ? "rotateX(10deg)" : "rotateX(160deg)",
            transition: leverPulled
              ? "transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)"
              : "transform 0.5s cubic-bezier(0.34, 1.56, 0.64, 1)",
          }}
        >
          {/* 3D arm — 4 faces of a rectangular prism */}
          <Lever3DArm />
          {/* 3D knob — sphere at the bottom of the arm */}
          <Lever3DKnob position="bottom" />
        </div>
      </div>
    </div>
  );
}

function ReelStrip({
  reel,
  type,
  finalValue,
  showEmoji,
  itemHeight,
  isDark,
}: {
  reel: { reelItems: string[]; offset: number; state: ReelState };
  type: ReelType;
  finalValue: string | undefined;
  showEmoji: boolean;
  itemHeight: number;
  isDark: boolean;
}) {
  const totalItems = reel.reelItems.length;
  const maxPixelOffset = totalItems * itemHeight;
  const wrappedOffset = maxPixelOffset > 0 ? reel.offset % maxPixelOffset : 0;

  // Cylinder geometry: radius so arc-distance between items ≈ itemHeight
  const angleRad = (CYLINDER_ITEM_ANGLE * Math.PI) / 180;
  const radius = itemHeight / (2 * Math.sin(angleRad / 2));

  // Current scroll position as fractional item index
  const currentPos = wrappedOffset / itemHeight;
  const halfVisible = Math.floor(CYLINDER_VISIBLE_ITEMS / 2);

  // Build visible items positioned around the cylinder
  const visibleItems: Array<{
    item: string;
    angle: number;
    slotKey: number;
    isWinner: boolean;
  }> = [];

  for (let i = -halfVisible; i <= halfVisible; i++) {
    let itemIdx = Math.floor(currentPos) + i;
    itemIdx = ((itemIdx % totalItems) + totalItems) % totalItems;
    const item = reel.reelItems[itemIdx];

    const frac = currentPos - Math.floor(currentPos);
    // Positive angle = rotated away at top, negative = rotated away at bottom
    const angle = (i - frac) * -CYLINDER_ITEM_ANGLE;

    const isWinner = reel.state === "stopped" && item === finalValue;
    visibleItems.push({ item, angle, slotKey: i, isWinner });
  }

  // CSS 3D cylinder: perspective is on the parent reel window div,
  // preserve-3d here lets items orbit a shared axis via rotateX + translateZ.
  return (
    <div
      style={{
        position: "absolute",
        width: "100%",
        height: 0,
        top: "50%",
        left: 0,
        transformStyle: "preserve-3d",
        // Push the cylinder axis back so the front face sits at the window surface
        transform: `translateZ(${-radius}px)`,
      }}
    >
      {visibleItems.map(({ item, angle, slotKey, isWinner }) => (
        <div
          key={slotKey}
          style={{
            position: "absolute",
            width: "100%",
            height: itemHeight,
            top: -itemHeight / 2,
            left: 0,
            transform: `rotateX(${angle}deg) translateZ(${radius}px)`,
            backfaceVisibility: "hidden",
          }}
        >
          <ReelItemContent
            item={item}
            type={type}
            showEmoji={showEmoji}
            itemHeight={itemHeight}
            isDark={isDark}
            isWinner={isWinner}
          />
        </div>
      ))}
    </div>
  );
}

/** Renders the content of a single reel item (emoji, text, or number) */
function ReelItemContent({
  item,
  type,
  showEmoji,
  itemHeight,
  isDark,
  isWinner,
}: {
  item: string;
  type: ReelType;
  showEmoji: boolean;
  itemHeight: number;
  isDark: boolean;
  isWinner: boolean;
}) {
  // Number reel: always big text, no emoji
  if (type === "number") {
    return (
      <div
        className="flex items-center justify-center font-mono tabular-nums"
        style={{
          height: itemHeight,
          fontSize: showEmoji ? 32 : 20,
          fontWeight: isWinner ? 800 : 600,
          color: isWinner ? BRAND.yellow : isDark ? "#e0e0e0" : "#333333",
          opacity: isWinner ? 1 : 0.75,
          textShadow: isWinner
            ? `0 0 12px ${BRAND.yellowGlow}, 0 0 24px ${BRAND.yellowGlow}`
            : "none",
        }}
      >
        {item}
      </div>
    );
  }

  // Emoji mode: icon on top, word below
  if (showEmoji) {
    const emoji = getEmoji(item, type);
    return (
      <div
        className="flex flex-col items-center justify-center gap-0"
        style={{
          height: itemHeight,
          opacity: isWinner ? 1 : 0.75,
        }}
      >
        <span style={{ fontSize: 28, lineHeight: 1 }}>{emoji}</span>
        <span
          className="font-mono tabular-nums"
          style={{
            fontSize: 10,
            fontWeight: isWinner ? 700 : 400,
            color: isWinner ? BRAND.yellow : isDark ? "#c0c0c0" : "#444444",
            textShadow: isWinner ? `0 0 8px ${BRAND.yellowGlow}` : "none",
            letterSpacing: "0.02em",
          }}
        >
          {item}
        </span>
      </div>
    );
  }

  // Text-only mode
  return (
    <div
      className={cn(
        "flex items-center justify-center font-mono text-sm tabular-nums",
        isWinner ? "font-bold" : "",
      )}
      style={{
        height: itemHeight,
        opacity: isWinner ? 1 : 0.75,
        color: isWinner ? BRAND.yellow : isDark ? "#e0e0e0" : "#333333",
        textShadow: isWinner ? `0 0 8px ${BRAND.yellowGlow}` : "none",
      }}
    >
      {item}
    </div>
  );
}

// ── 3D Lever parts ──────────────────────────────────────────────────────────

const ARM_W = 8; // width (x)
const ARM_D = 8; // depth (z)
const HALF_W = ARM_W / 2;
const HALF_D = ARM_D / 2;

/** A rectangular prism (the lever rod) built from 4 CSS 3D faces */
function Lever3DArm() {
  const face: React.CSSProperties = {
    position: "absolute",
    top: 0,
    left: 0,
    width: ARM_W,
    height: "100%",
    borderRadius: 2,
  };
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        transformStyle: "preserve-3d",
      }}
    >
      {/* Front */}
      <div
        style={{
          ...face,
          transform: `translateZ(${HALF_D}px)`,
          background: "linear-gradient(180deg, #999 0%, #777 50%, #888 100%)",
        }}
      />
      {/* Back */}
      <div
        style={{
          ...face,
          transform: `translateZ(-${HALF_D}px)`,
          background: "linear-gradient(180deg, #555 0%, #333 50%, #444 100%)",
        }}
      />
      {/* Right */}
      <div
        style={{
          ...face,
          width: ARM_D,
          transform: `rotateY(90deg) translateZ(${HALF_W}px)`,
          background: "linear-gradient(180deg, #888 0%, #666 50%, #777 100%)",
        }}
      />
      {/* Left */}
      <div
        style={{
          ...face,
          width: ARM_D,
          transform: `rotateY(-90deg) translateZ(${HALF_W}px)`,
          background: "linear-gradient(180deg, #666 0%, #444 50%, #555 100%)",
        }}
      />
    </div>
  );
}

/**
 * A sphere built from horizontal disc slices at different Z depths.
 * Each slice is a circle whose diameter = 2 * sqrt(R² - z²).
 */
const KNOB_R = 11; // radius 11px → 22px diameter
const KNOB_SLICES = [-9, -6, -3, 0, 3, 6, 9].map((z) => {
  const d = 2 * Math.sqrt(KNOB_R * KNOB_R - z * z);
  return { z, d: Math.round(d) };
});

function Lever3DKnob({ position }: { position: "top" | "bottom" }) {
  return (
    <div
      style={{
        position: "absolute",
        ...(position === "top" ? { top: -KNOB_R } : { bottom: -KNOB_R }),
        left: "50%",
        width: 0,
        height: 0,
        transformStyle: "preserve-3d",
      }}
    >
      {KNOB_SLICES.map(({ z, d }) => {
        // Shade from darker (back) to lighter (front), with a raised floor
        // so the sphere stays vibrant even when viewed from the back (resting state)
        const t = (z + 9) / 18; // 0 (back) → 1 (front)
        const lightness = Math.round(50 + t * 20); // 50%–70%
        return (
          <div
            key={z}
            style={{
              position: "absolute",
              width: d,
              height: d,
              left: -d / 2,
              top: -d / 2,
              borderRadius: "50%",
              transform: `translateZ(${z}px)`,
              background:
                z === 0
                  ? `radial-gradient(circle at 38% 32%, #ffa090, #ee3030 40%, #dd2828 75%, #cc2222 100%)`
                  : `hsl(4, 80%, ${lightness}%)`,
              boxShadow: z === 0 ? `0 2px 6px ${BRAND.redGlow}` : undefined,
            }}
          />
        );
      })}
    </div>
  );
}

function reelWidth(type: ReelType, showEmoji: boolean): number {
  if (showEmoji) {
    switch (type) {
      case "adjective":
        return 110;
      case "animal":
        return 110;
      case "number":
        return 80;
      default:
        return 100;
    }
  }
  switch (type) {
    case "adjective":
      return 130;
    case "animal":
      return 120;
    case "number":
      return 50;
    default:
      return 100;
  }
}
