import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import vue from "eslint-plugin-vue";
import vueParser from "vue-eslint-parser";
import prettier from "eslint-config-prettier";

export default [
    {
        ignores: [
            "dist/**",
            "node_modules/**",
            "src-tauri/target/**",
            "src-tauri/gen/**",
            "public/build/**",
            "nativephp/**",
            "vendor/**",
            "storage/**",
            "bootstrap/**",
            "database/**",
            "resources/**",
        ],
    },
    js.configs.recommended,
    ...tseslint.configs.recommended,
    ...vue.configs["flat/recommended"],
    {
        languageOptions: {
            globals: { ...globals.browser, ...globals.es2024 },
        },
    },
    {
        files: ["**/*.vue"],
        languageOptions: {
            parser: vueParser,
            parserOptions: {
                parser: tseslint.parser,
                ecmaVersion: "latest",
                sourceType: "module",
            },
        },
    },
    {
        rules: {
            "vue/multi-word-component-names": "off",
            "@typescript-eslint/no-unused-vars": [
                "warn",
                { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
            ],
        },
    },
    prettier,
];
