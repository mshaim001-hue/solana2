/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}"],
  theme: {
    extend: {
      colors: {
        ink: "#0c1222",
        mist: "#e8eef7",
        accent: "#1f8a70",
        warn: "#c45c26",
        line: "#243044",
      },
      fontFamily: {
        display: ["var(--font-display)", "Georgia", "serif"],
        sans: ["var(--font-sans)", "ui-sans-serif", "system-ui"],
      },
    },
  },
  plugins: [],
};
