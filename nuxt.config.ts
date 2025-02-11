// https://nuxt.com/docs/api/configuration/nuxt-config
import {definePreset} from "@primevue/themes";
import Aura from "@primevue/themes/aura";

const AutonomiPreset = definePreset(Aura, {
    // Customizations
    // semantic: {
    // button: {
    //   border: {
    //     radius: "10rem",
    //   },
    // },
    // },
    semantic: {
        button: {
            background: {
                color: "black",
            },
        },
    },
    colorScheme: {
        light: {
            primary: {
                color: "#000000",
            },
        },
        dark: {
            primary: {
                color: "#FF0000",
            },
        },
    },
});

export default defineNuxtConfig({
    compatibilityDate: "2024-11-01",
    // (optional) Enable the Nuxt devtools
    devtools: {enabled: true},
    // Enable SSG
    ssr: false,
    // Enables the development server to be discoverable by other devices when running on iOS physical devices
    devServer: {host: process.env.TAURI_DEV_HOST || "localhost"},
    vite: {
        // Better support for Tauri CLI output
        clearScreen: false,
        // Enable environment variables
        // Additional environment variables can be found at
        // https://v2.tauri.app/reference/environment-variables/
        envPrefix: ["VITE_", "TAURI_"],
        server: {
            // Tauri requires a consistent port
            strictPort: true,
        },
    },
    css: ["~/assets/css/main.css"],
    postcss: {
        plugins: {
            tailwindcss: {},
            autoprefixer: {},
        },
    },
    modules: ["@primevue/nuxt-module", "@pinia/nuxt"],
    primevue: {
        options: {
            ripple: true,
            inputVariant: "filled",
            theme: {
                preset: AutonomiPreset,
                options: {
                    prefix: "p",
                    darkModeSelector: "autonomi-dark",
                    cssLayer: {
                        name: "primevue",
                        order: "tailwind-base, primevue, tailwind-utilities"
                    },
                },
            },
        },
    },
});
