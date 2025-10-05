import { defineConfig, globalIgnores } from 'eslint/config';
import tsParser from '@typescript-eslint/parser';
import typescriptEslint from '@typescript-eslint/eslint-plugin';
import globals from 'globals';
import parser from 'svelte-eslint-parser';
import js from '@eslint/js';
import { FlatCompat } from '@eslint/eslintrc';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
	baseDirectory: __dirname,
	recommendedConfig: js.configs.recommended,
	allConfig: js.configs.all
});

export default defineConfig([
	{
		extends: compat.extends(
			'eslint:recommended',
			'plugin:@typescript-eslint/recommended',
			'plugin:svelte/recommended',
			'prettier'
		),

		languageOptions: {
			parser: tsParser,
			sourceType: 'module',
			ecmaVersion: 2020,

			parserOptions: {
				extraFileExtensions: ['.svelte']
			},

			globals: {
				...globals.browser,
				...globals.node
			}
		},

		plugins: {
			'@typescript-eslint': typescriptEslint
		}
	},
	{
		files: ['**/*.svelte'],

		languageOptions: {
			parser: parser,

			parserOptions: {
				parser: '@typescript-eslint/parser'
			}
		}
	},
	globalIgnores([
		'**/.DS_Store',
		'**/node_modules',
		'build',
		'.svelte-kit',
		'package',
		'**/.env',
		'**/.env.*',
		'!**/.env.example',
		'**/pnpm-lock.yaml',
		'**/package-lock.json',
		'**/yarn.lock'
	])
]);
