import { createContext, useContext } from 'react';
import english from '../../../locales/en.json';

/** What a file of `locales/` holds: the language's details and its texts. */
interface Locale {
  language: { name: string; decimal: string; intervals: string; quadratic: string };
  texts: Record<string, string>;
}

/** Every file of `locales/`, by its code (`en`, `de`). */
const LOCALES: Record<string, Locale> = Object.fromEntries(
  Object.entries(import.meta.glob<Locale>('../../../locales/*.json', { eager: true, import: 'default' })).map(([path, locale]) => [
    path.slice(path.lastIndexOf('/') + 1, -'.json'.length),
    locale,
  ]),
);

/** A language's code, the name of its file in `locales/`. */
export type Language = string;

/** The frontend's texts are the `app.` keys of `locales/en.json`, named here without the prefix. */
export type TextKey = keyof typeof english.texts extends infer K ? (K extends `app.${infer Key}` ? Key : never) : never;

/** English first, the fallback; then the others by code. */
export const LANGUAGES: Language[] = Object.keys(LOCALES).sort((a, b) => (a === 'en' ? -1 : b === 'en' ? 1 : a.localeCompare(b)));

/** Native names, shown the same in every language. */
export const LANGUAGE_NAMES: Record<Language, string> = Object.fromEntries(LANGUAGES.map((code) => [code, LOCALES[code].language.name]));

export type Translate = (key: TextKey, values?: Record<string, string | number>) => string;

const ENGLISH: Record<string, string> = english.texts;

/** Looks a text up, in English where the language lacks it, and fills its `{name}` placeholders. */
export function makeTranslate(language: Language): Translate {
  const texts = LOCALES[language]?.texts ?? ENGLISH;
  return (key, values) => {
    const value = texts[`app.${key}`] || ENGLISH[`app.${key}`] || key;
    return value.replace(/\{(\w+)\}/g, (match, name: string) => (name in (values ?? {}) ? String(values![name]) : match));
  };
}

export const I18nContext = createContext<{ language: Language; t: Translate }>({
  language: 'en',
  t: makeTranslate('en'),
});

export const useT = () => useContext(I18nContext);
