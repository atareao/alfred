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
      // La API estática de antd crea el nodo del aviso fuera del árbol de React
      // y su setTimeout sobrevive al desmontaje. Usar `App.useApp()` en su lugar.
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'antd',
              importNames: ['message', 'notification'],
              message:
                'Usa `App.useApp()`: la API estática de antd vive fuera del ciclo de vida de React.',
            },
          ],
          patterns: [
            {
              group: ['antd/es/*', 'antd/lib/*'],
              message:
                'Importa desde "antd", no por rutas internas del paquete.',
            },
          ],
        },
      ],
      // Los estáticos de `Modal` tienen la misma fuga de temporizador que
      // `message`/`notification`: crean el nodo del aviso fuera del árbol de
      // React y su `setTimeout` sobrevive al desmontaje. Usar `App.useApp().modal`.
      'no-restricted-syntax': [
        'error',
        {
          selector:
            'MemberExpression[object.name=/Modal$/][property.name=/^(confirm|info|success|error|warning|destroy)$/]',
          message:
            'Los estáticos de `Modal` crean el nodo fuera del árbol de React y su temporizador sobrevive al desmontaje. Usa `App.useApp().modal`.',
        },
      ],
    },
  },
)
