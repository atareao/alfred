import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'

export default tseslint.config(
  {
    ignores: ['dist/', 'node_modules/', 'coverage/'],
  },
  ...tseslint.configs.recommended,
  reactHooks.configs.flat['recommended-latest'],
  {
    files: ['**/*.{ts,tsx}'],
    plugins: {
      'react-refresh': reactRefresh,
    },
    rules: {
      'react-refresh/only-export-components': 'warn',
      // Convención del proyecto: parámetros/variables intencionalmente sin usar
      // se prefijan con "_" (p. ej. callbacks de antd que ignoran el evento).
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
      // Regla nueva del React Compiler (react-hooks v7). El código existente
      // llama a loaders/refetch dentro de useEffect; degradada a warning para
      // no forzar refactors masivos fuera del alcance de este change.
      'react-hooks/set-state-in-effect': 'warn',
    },
  },
)
