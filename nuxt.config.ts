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
    devtools: {enabled: false},
    // Enable SSG
    ssr: false,
    // Enables the development server to be discoverable by other devices when running on iOS physical devices
    devServer: {host: process.env.TAURI_DEV_HOST || "localhost"},
    ignore: [
        '**/src-tauri/**',
        '**/.git/**',
        '**/node_modules/**',
        '**/.nuxt/**',
        '**/.output/**',
        '**/dist/**',
        '**/*.log',
        '**/.DS_Store',
        '**/Thumbs.db'
    ],
    watchers: {
        webpack: {
            ignored: /node_modules/
        },
        chokidar: {
            ignoreInitial: true,
            followSymlinks: false,
            usePolling: false,
            interval: 1000,
            binaryInterval: 1000,
            awaitWriteFinish: {
                stabilityThreshold: 500,
                pollInterval: 100
            }
        }
    },
    vite: {
        plugins: [],
        // Better support for Tauri CLI output
        clearScreen: false,
        // Enable environment variables
        // Additional environment variables can be found at
        // https://v2.tauri.app/reference/environment-variables/
        envPrefix: ["VITE_", "TAURI_"],
        server: {
            port: 1420,
            strictPort: true,
            host: process.env.TAURI_DEV_HOST || false,
            hmr: process.env.TAURI_DEV_HOST
                ? {
                    protocol: "ws",
                    host: process.env.TAURI_DEV_HOST,
                    port: 1421,
                }
                : undefined,
            watch: {
                // Tell vite to ignore watching `src-tauri`
                ignored: ["**/src-tauri/**"],
            },
        },
    },
    css: ["~/assets/css/main.css"],
    postcss: {
        plugins: {
            tailwindcss: {},
            autoprefixer: {},
        },
    },
    runtimeConfig: {
        public: {
            devVaultSignature: process.env.VITE_DEV_VAULT_SIGNATURE || '',
            pimlicoApiKey: process.env.VITE_PIMLICO_API_KEY || ''
        }
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
