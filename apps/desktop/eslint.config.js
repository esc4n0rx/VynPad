import js from "@eslint/js";
import globals from "globals";
import i18next from "eslint-plugin-i18next";
import jsxA11y from "eslint-plugin-jsx-a11y";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

export default tseslint.config(
  // `bindings.ts` é gerado pelo tauri-specta (src-tauri/src/lib.rs) e não se edita à mão.
  { ignores: ["dist", "src-tauri", "scripts", "src/lib/bindings.ts"] },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [
      js.configs.recommended,
      ...tseslint.configs.recommendedTypeChecked,
      reactHooks.configs.flat["recommended-latest"],
      reactRefresh.configs.vite,
      jsxA11y.flatConfigs.recommended,
    ],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
      "@typescript-eslint/consistent-type-imports": "error",
    },
  },
  {
    // Cor só vem dos tokens (`index.css`): sem hex/rgb/hsl nos componentes. Exceções: a lógica de
    // tema (accent padrão), a vitrine `/dev/ui` (amostras de accent) e os testes.
    files: ["src/**/*.{ts,tsx}"],
    ignores: ["src/lib/theme.ts", "src/features/dev/**", "src/**/*.test.{ts,tsx}"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector: "Literal[value=/#[0-9a-fA-F]{3,8}\\b|\\b(rgb|hsl|oklch|oklab)a?\\(/]",
          message: "Cor hardcoded: use um token do design system (index.css).",
        },
        {
          selector:
            "TemplateElement[value.raw=/#[0-9a-fA-F]{3,8}\\b|\\b(rgb|hsl|oklch|oklab)a?\\(/]",
          message: "Cor hardcoded: use um token do design system (index.css).",
        },
      ],
    },
  },
  {
    // Texto visível sai dos arquivos de idioma (`src/i18n`), não do JSX. A vitrine /dev/ui e os
    // testes ficam de fora.
    files: ["src/**/*.tsx"],
    ignores: ["src/features/dev/**", "src/**/*.test.tsx"],
    extends: [i18next.configs["flat/recommended"]],
    rules: {
      "i18next/no-literal-string": [
        "error",
        {
          mode: "jsx-only",
          words: { exclude: ["PadGaming", "[0-9\s.,:;·/|()+\-–—•…]*", "P[1-4]"] },
        },
      ],
    },
  },
  {
    // Arquivos de configuração rodam no Node e ficam fora do projeto TS do app.
    files: ["*.config.{js,ts}"],
    extends: [tseslint.configs.disableTypeChecked],
    languageOptions: { globals: globals.node },
  },
);
