/** @type {import('tailwindcss').Config} */
const colors = require("tailwindcss/colors");
const autonomiColors = {
    red: {
        300: "#FF3355",
    },
    blue: {
        50: "#F3F3FB", // very light tint
        100: "#D7D7F7",
        200: "#A9A9F2", // given
        300: "#7D7DEB",
        400: "#5555E0",
        500: "#3A3AC8",
        600: "#26264C", // given (a dark desaturated indigo)
        700: "#1A1A45",
        800: "#11113B", // given
        900: "#0A0A2D",
    },
    gray: {
        50: "#F8F8FA",
        100: "#F2F2F6",
        200: "#E4E7EC",
        300: "#CCCCCC",
        500: "#8994A3",
        600: "#666666",
    },
    green: {
        300: "#60C281",
    },
};

export default {
    content: [
        "./components/**/*.{js,vue,ts}",
        "./layouts/**/*.vue",
        "./pages/**/*.vue",
        "./plugins/**/*.{js,ts}",
        "./app.vue",
        "./error.vue",
    ],
    theme: {
        colors: {
            ...colors,
            autonomi: {
                ...autonomiColors,
                header: {
                    text: {
                        DEFAULT: autonomiColors.blue[600],
                        dark: autonomiColors.blue[800],
                    },
                },
                text: {
                    primary: {
                        DEFAULT: autonomiColors.gray[500],
                        dark: autonomiColors.gray[300],
                    },
                    secondary: {
                        DEFAULT: autonomiColors.blue[600],
                        dark: autonomiColors.gray[300],
                    },
                },
            },
        },
        extend: {},
    },
    darkMode: ["class"],
    plugins: [require("tailwindcss-primeui")]
};
