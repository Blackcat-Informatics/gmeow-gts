// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import tseslint from "@typescript-eslint/eslint-plugin";
import tsparser from "@typescript-eslint/parser";

export default [
    {
        files: ["src/**/*.ts", "tests/**/*.ts"],
        languageOptions: {
            parser: tsparser,
            parserOptions: {
                project: ["./tsconfig.json", "./tsconfig.test.json"],
            },
        },
        plugins: {
            "@typescript-eslint": tseslint,
        },
        rules: {
            ...tseslint.configs.recommended.rules,
            "@typescript-eslint/await-thenable": "error",
            "@typescript-eslint/no-misused-promises": "error",
            "@typescript-eslint/no-unnecessary-type-assertion": "error",
            "@typescript-eslint/no-unused-vars": [
                "error",
                { argsIgnorePattern: "^_" },
            ],
            "@typescript-eslint/switch-exhaustiveness-check": "error",

            // Evaluated during issue #143 but intentionally deferred:
            // - no-floating-promises: Node test() calls intentionally register tests
            //   by returning a Promise-like value at top level; enabling this cleanly
            //   needs a test harness convention pass.
            // - no-unnecessary-condition: dynamic CBOR/path guards still produce
            //   useful runtime defense even where static narrowing thinks otherwise.
            // - restrict-template-expressions: useful, but currently needs a focused
            //   diagnostic-formatting cleanup for unknown values.
        },
    },
];
