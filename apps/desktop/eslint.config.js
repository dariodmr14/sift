// ESLint 10 usa "flat config": un array de objetos de configuración que se aplican en orden.
import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import prettier from "eslint-config-prettier";

export default tseslint.config(
  { ignores: ["dist", "src-tauri"] },
  js.configs.recommended,
  // Reglas que usan la información de tipos de TypeScript (detectan p. ej. promesas sin await).
  ...tseslint.configs.recommendedTypeChecked,
  {
    languageOptions: {
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  reactHooks.configs.flat.recommended,
  // Los .js de configuración no forman parte del proyecto TS: sin reglas con tipos.
  { files: ["**/*.js"], ...tseslint.configs.disableTypeChecked },
  // Siempre la última: desactiva las reglas de estilo que chocarían con Prettier.
  prettier,
);
