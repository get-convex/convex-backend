/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: "class",
  content: ["./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    boxShadow: {
      // Custom shadow
      1: "0px 4px 4px rgba(0, 0, 0, 0.03)",
      // We're using the Material Design shadow system (https://getcssscan.com/css-box-shadow-examples)
      2: "rgba(0, 0, 0, 0.16) 0px 3px 6px, rgba(0, 0, 0, 0.23) 0px 3px 6px",
      3: "rgba(0, 0, 0, 0.19) 0px 10px 20px, rgba(0, 0, 0, 0.23) 0px 6px 6px",
      4: "rgba(0, 0, 0, 0.25) 0px 14px 28px, rgba(0, 0, 0, 0.22) 0px 10px 10px",
      5: "rgba(0, 0, 0, 0.3) 0px 19px 38px, rgba(0, 0, 0, 0.22) 0px 15px 12px",
    },
    extend: {
      animation: {
        blink: "blink 2s linear infinite",
        bounceIn: "bounceIn 0.5s ease-in-out",
        highlight: "highlight 1s",
        highlightDark: "highlightDark 1s",
        loading: "fadeIn 1.5s, shimmer 1.5s infinite",
        fadeIn: "fadeIn 2s",
        fadeInFromLoading: "fadeIn 0.2s",
        shake: "shake 0.5s infinite",
      },
      keyframes: {
        shake: {
          "0%": { transform: "translate(1px, 1px) rotate(0deg)" },
          "10%": { transform: "translate(-1px, -2px) rotate(-1deg)" },
          "20%": { transform: "translate(-3px, 0px) rotate(1deg)" },
          "30%": { transform: "translate(3px, 2px) rotate(0deg)" },
          "40%": { transform: "translate(1px, -1px) rotate(1deg)" },
          "50%": { transform: "translate(-1px, 2px) rotate(-1deg)" },
          "60%": { transform: "translate(-3px, 1px) rotate(0deg)" },
          "70%": { transform: "translate(3px, 1px) rotate(-1deg)" },
          "80%": { transform: "translate(-1px, -1px) rotate(1deg)" },
          "90%": { transform: "translate(1px, 2px) rotate(0deg)" },
          "100%": { transform: "translate(1px, -2px) rotate(-1deg)" },
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
        bounceIn: {
          "0%": { transform: "translateY(0);" },
          "25%": { transform: "translateY(-0.5rem);" },
          "50%": { transform: "translateY(0px);" },
          "75%": { transform: "translateY(-0.25rem);" },
          "100%": { transform: "translateY(0px);" },
        },
        highlight: {
          "0%": { backgroundColor: "rgb(251, 252, 253)" },
          "50%": { backgroundColor: "rgb(203, 237, 182)" },
          "100%": { backgroundColor: "rgb(251, 252, 253)" },
        },
        highlightDark: {
          "0%": { backgroundColor: "rgb(37, 39, 46)" },
          "50%": { backgroundColor: "rgb(44, 83, 20)" },
          "100%": { backgroundColor: "rgb(37, 39, 46)" },
        },
        fadeIn: {
          "0%": {
            opacity: 0,
          },
          "100%": {
            opacity: 1,
          },
        },
      },
      colors: {
        util: {
          accent: "rgb(38, 135, 246)",
          info: "rgb(7, 191, 232)",
          success: "rgb(79, 176, 20)",
          danger: "rgb(238, 52, 47)",
          warning: "rgb(243, 176, 28)",
        },
        light: {
          background: {
            primary: "rgb(240, 242, 246)",
            secondary: "rgb(251, 252, 255)",
            tertiary: "rgb(232, 236, 244)",
          },
          content: {
            primary: "rgb(32, 36, 41)",
            secondary: "rgb(70, 79, 87)",
            tertiary: "rgb(116, 125, 136)",
            accent: "rgb(48, 106, 207)",
          },
          border: {
            transparent: "rgba(13, 34, 109, 0.14)",
            selected: "rgb(20, 20, 20)",
          },
        },
        dark: {
          background: {
            primary: "rgb(24, 25, 28)",
            secondary: "rgb(37, 39, 46)",
            tertiary: "rgb(55, 57, 71)", // Hover background
          },
          content: {
            primary: "rgb(255, 255, 255)",
            secondary: "rgb(177, 185, 192)",
            tertiary: "rgb(133, 139, 153)",
            accent: "rgb(99, 168, 248)",
          },
          border: {
            transparent: "rgba(153, 176, 198, 0.30)",
            selected: "rgb(215, 215, 215)",
          },
        },
        red: {
          100: "rgb(252, 215, 203)",
          200: "rgb(255, 202, 193)",
          400: "rgb(253, 76, 65)",
          500: "rgb(238, 52, 47)", // Error text color
          700: "rgb(168, 21, 21)",
          900: "rgb(107, 33, 31)",
        },
        purple: {
          100: "rgb(241, 200, 233)",
          200: "rgb(232, 180, 220)",
          500: "rgb(141, 38, 118)",
          700: "rgb(86, 0, 83)",
          900: "rgb(113, 36, 96)",
        },
        blue: {
          100: "rgb(204, 222, 250)",
          200: "rgb(177, 202, 240)",
          500: "rgb(7, 78, 232)",
          700: "rgb(33, 34, 181)",
          900: "rgb(0, 43, 153)",
        },
        cyan: {
          200: "rgb(170, 227, 239)",
          500: "rgb(7, 192, 232)",
          700: "rgb(0, 155, 221)",
          900: "rgb(15, 89, 105)",
        },
        green: {
          100: "rgb(203, 237, 182)",
          200: "rgb(180, 236, 146)",
          500: "rgb(79, 176, 20)",
          700: "rgb(34, 137, 9)",
          900: "rgb(44, 83, 20)",
        },
        yellow: {
          100: "rgb(250, 228, 171)",
          200: "rgb(230, 226, 168)",
          500: "rgb(243, 176, 28)",
          700: "rgb(213, 113, 21)",
          900: "rgb(109, 82, 23)",
        },
        neutral: {
          1: "rgb(222, 226, 234)",
          2: "rgb(204, 206, 211)",
          3: "rgb(174, 177, 184)",
          4: "rgb(151, 154, 164)",
          5: "rgb(133, 136, 147)",
          6: "rgb(118, 121, 131)",
          7: "rgb(103, 106, 116)",
          8: "rgb(88, 92, 101)",
          9: "rgb(73, 76, 84)",
          10: "rgb(57, 60, 66)",
          11: "rgb(41, 43, 48)",
          12: "rgb(24, 25, 28)",
        },
      },
      fontFamily: {
        display:
          '"Saira", system-ui, "Segoe UI", Roboto, Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"',
      },
    },
  },
  plugins: [],
};
