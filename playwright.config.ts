import { defineConfig } from "@playwright/test"

export default defineConfig({
    testDir: "./e2e",
    testMatch: "**/*.test.ts",
    reporter: process.env.CI ? [
        ["github"],
        ["html", { open: "never" }]
    ] : undefined,
})
