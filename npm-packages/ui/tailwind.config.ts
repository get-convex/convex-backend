export const numberedColors = {
  red: {
    /* eslint-disable no-restricted-syntax */
    100: `rgba(var(--red-100), <alpha-value>)`,
    200: `rgba(var(--red-200), <alpha-value>)`,
    300: `rgba(var(--red-300), <alpha-value>)`,
    400: `rgba(var(--red-400), <alpha-value>)`,
    500: `rgba(var(--red-500), <alpha-value>)`,
    700: `rgba(var(--red-700), <alpha-value>)`,
    900: `rgba(var(--red-900), <alpha-value>)`,
  },
  purple: {
    100: `rgba(var(--purple-100), <alpha-value>)`,
    200: `rgba(var(--purple-200), <alpha-value>)`,
    500: `rgba(var(--purple-500), <alpha-value>)`,
    700: `rgba(var(--purple-700), <alpha-value>)`,
    900: `rgba(var(--purple-900), <alpha-value>)`,
  },
  blue: {
    100: `rgba(var(--blue-100), <alpha-value>)`,
    200: `rgba(var(--blue-200), <alpha-value>)`,
    500: `rgba(var(--blue-500), <alpha-value>)`,
    700: `rgba(var(--blue-700), <alpha-value>)`,
    900: `rgba(var(--blue-900), <alpha-value>)`,
  },
  cyan: {
    200: `rgba(var(--cyan-200), <alpha-value>)`,
    500: `rgba(var(--cyan-500), <alpha-value>)`,
    700: `rgba(var(--cyan-700), <alpha-value>)`,
    900: `rgba(var(--cyan-900), <alpha-value>)`,
  },
  green: {
    100: `rgba(var(--green-100), <alpha-value>)`,
    200: `rgba(var(--green-200), <alpha-value>)`,
    500: `rgba(var(--green-500), <alpha-value>)`,
    700: `rgba(var(--green-700), <alpha-value>)`,
    900: `rgba(var(--green-900), <alpha-value>)`,
  },
  yellow: {
    50: `rgba(var(--yellow-50), <alpha-value>)`,
    100: `rgba(var(--yellow-100), <alpha-value>)`,
    200: `rgba(var(--yellow-200), <alpha-value>)`,
    500: `rgba(var(--yellow-500), <alpha-value>)`,
    700: `rgba(var(--yellow-700), <alpha-value>)`,
    900: `rgba(var(--yellow-900), <alpha-value>)`,
  },
  neutral: {
    1: `rgba(var(--neutral-1), <alpha-value>)`,
    2: `rgba(var(--neutral-2), <alpha-value>)`,
    3: `rgba(var(--neutral-3), <alpha-value>)`,
    4: `rgba(var(--neutral-4), <alpha-value>)`,
    5: `rgba(var(--neutral-5), <alpha-value>)`,
    6: `rgba(var(--neutral-6), <alpha-value>)`,
    7: `rgba(var(--neutral-7), <alpha-value>)`,
    8: `rgba(var(--neutral-8), <alpha-value>)`,
    9: `rgba(var(--neutral-9), <alpha-value>)`,
    10: `rgba(var(--neutral-10), <alpha-value>)`,
    11: `rgba(var(--neutral-11), <alpha-value>)`,
    12: `rgba(var(--neutral-12), <alpha-value>)`,
  },
};

type ThemeColors = {
  background: {
    brand: string;
    primary: string;
    secondary: string;
    tertiary: string;
    success: string;
    warning: string;
    error: string;
    errorSecondary: string;
  };
  chart: {
    line: {
      1: string;
      2: string;
      3: string;
      4: string;
      5: string;
      6: string;
      7: string;
      8: string;
    };
  };
  content: {
    primary: string;
    secondary: string;
    tertiary: string;
    accent: string;
    success: string;
    warning: string;
    error: string;
    errorSecondary: string;
    link: string;
  };
  border: {
    transparent: string;
    selected: string;
  };
};

export const themeColors: ThemeColors = {
  background: {
    brand: "rgba(var(--background-brand), <alpha-value>)",
    primary: `rgba(var(--background-primary), <alpha-value>)`,
    secondary: `rgba(var(--background-secondary), <alpha-value>)`,
    tertiary: `rgba(var(--background-tertiary), <alpha-value>)`,
    success: `rgba(var(--background-success), <alpha-value>)`,
    warning: `rgba(var(--background-warning), <alpha-value>)`,
    error: `rgba(var(--background-error), <alpha-value>)`,
    errorSecondary: `rgba(var(--background-error-secondary), <alpha-value>)`,
  },
  content: {
    primary: `rgba(var(--content-primary), <alpha-value>)`,
    secondary: `rgba(var(--content-secondary), <alpha-value>)`,
    tertiary: `rgba(var(--content-tertiary), <alpha-value>)`,
    accent: `rgba(var(--content-accent), <alpha-value>)`,
    success: `rgba(var(--content-success), <alpha-value>)`,
    warning: `rgba(var(--content-warning), <alpha-value>)`,
    error: `rgba(var(--content-error), <alpha-value>)`,
    errorSecondary: `rgba(var(--content-error-secondary), <alpha-value>)`,
    link: `rgba(var(--content-link), <alpha-value>)`,
  },
  chart: {
    line: {
      1: `rgba(var(--chart-line-1), <alpha-value>)`,
      2: `rgba(var(--chart-line-2), <alpha-value>)`,
      3: `rgba(var(--chart-line-3), <alpha-value>)`,
      4: `rgba(var(--chart-line-4), <alpha-value>)`,
      5: `rgba(var(--chart-line-5), <alpha-value>)`,
      6: `rgba(var(--chart-line-6), <alpha-value>)`,
      7: `rgba(var(--chart-line-7), <alpha-value>)`,
      8: `rgba(var(--chart-line-8), <alpha-value>)`,
    },
  },
  border: {
    // Transparent border color already has it's own alpha.
    transparent: `rgba(var(--border-transparent))`,
    selected: `rgba(var(--border-selected), <alpha-value>)`,
  },
};

export const utilColors = {
  accent: `rgba(var(--accent), <alpha-value>)`,
  info: `rgba(var(--info), <alpha-value>)`,
  success: `rgba(var(--success), <alpha-value>)`,
  warning: `rgba(var(--warning), <alpha-value>)`,
  danger: `rgba(var(--error), <alpha-value>)`,
  brand: {
    purple: `rgba(var(--brand-purple), <alpha-value>)`,
    red: `rgba(var(--brand-red), <alpha-value>)`,
    yellow: `rgba(var(--brand-yellow), <alpha-value>)`,
  },
};

// eslint-disable-next-line import/no-default-export
export default {
  darkMode: "class",
  content: [
    "../ui/src/**/*.{js,ts,jsx,tsx}",
    "../ui/src/*.{js,ts,jsx,tsx}",
    "../dashboard/src/**/*.{js,ts,jsx,tsx}",
    "../dashboard-common/src/**/*.{js,ts,jsx,tsx}",
    "../dashboard-self-hosted/src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      animation: {
        blink: "blink 2s linear infinite",
        bounceIn: "bounceIn 0.5s ease-in-out",
        highlight: "highlight 1s",
        highlightBorder: "highlightBorder 1s",
        loading: "fadeIn 1.2s, shimmer 1.2s infinite",
        fadeIn: "fadeIn 1s",
        fadeInFromLoading: "fadeIn 0.3s",
        vhs: "vhs 0.5s linear 0.25s 1 normal forwards",
        blinkFill: "blinkFill 1.2s ease-in-out infinite",
        rotate: "fadeIn 1.2s, rotate 1.2s ease-in-out infinite",
      },
      keyframes: {
        rotate: {
          "0%": { transform: "rotate(0deg)" },
          "100%": { transform: "rotate(360deg)" },
        },
        blinkFill: {
          "0%": {
            fillOpacity: 1,
            opacity: 1,
          },
          "50%": {
            fillOpacity: 0.75,
            opacity: 0.75,
          },
          "100%": {
            fillOpacity: 1,
            opacity: 1,
          },
        },
        shimmer: {
          "100%": {
            transform: "translateX(100%)",
          },
        },
        blink: {
          "0%": {
            opacity: 1,
          },
          "50%": {
            opacity: 0.5,
          },
          "100%": {
            opacity: 1,
          },
        },
        vhs: {
          "0%": {
            height: "0%",
            transform: "skew(-90deg)",
            marginLeft: "-2rem",
          },
          "100%": {
            height: "100%",
            transform: "skew(0deg)",
            marginRight: "0px",
          },
        },
        bounceIn: {
          "0%": { transform: "translateY(0);" },
          "25%": { transform: "translateY(-0.5rem);" },
          "50%": { transform: "translateY(0px);" },
          "75%": { transform: "translateY(-0.25rem);" },
          "100%": { transform: "translateY(0px);" },
        },
        highlight: {
          // <alpha-value> does not get propogated within tailwind config, so we have to specify values here
          // instead of using the constants above
          "0%": { backgroundColor: "rgb(var(--background-secondary))" },
          "50%": { backgroundColor: "rgb(var(--background-highlight))" },
          "100%": { backgroundColor: "rgb(var(--background-secondary))" },
        },
        highlightBorder: {
          // <alpha-value> does not get propogated within tailwind config, so we have to specify values here
          // instead of using the constants above
          "0%": { backgroundColor: "rgb(var(--border-transparent))" },
          "50%": { backgroundColor: "rgb(var(--content-success))" },
          "100%": { backgroundColor: "rgb(var(--border-transparent))" },
        },
        fadeIn: {
          "0%": {
            opacity: 0,
          },
          "100%": {
            opacity: 1,
          },
        },
        indeterminateProgressBar: {
          "0%": { transform: "none" },
          "100%": { transform: "translateX(-1rem)" },
        },
      },
      colors: {
        util: utilColors,
        ...themeColors,
        ...numberedColors,
      },
      fontFamily: {
        marketing: [
          "GT America",
          "Inter Variable",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "Arial",
          "Noto Sans",
          "sans-serif",
          "Apple Color Emoji",
          "Segoe UI Emoji",
          "Segoe UI Symbol",
          "Noto Color Emoji",
          "sans-serif",
        ],
      },
    },
  },
  plugins: [
    require("tailwind-scrollbar")({ nocompatible: true }),
    require("@tailwindcss/forms")({ strategy: "class" }),
  ],
};
