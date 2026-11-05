import { createContext, useContext } from 'react';
import file from '../../../texts/texts.json';

/** The languages of `texts/texts.json`. */
export type Language = keyof typeof file.languages;

/** The frontend's texts are the `app.` keys of `texts/texts.json`, named here without the prefix. */
export type TextKey = keyof typeof file.texts extends infer K ? (K extends `app.${infer Key}` ? Key : never) : never;

const TEXTS: Record<string, Partial<Record<Language, string>>> = file.texts;

export const LANGUAGES = Object.keys(file.languages) as Language[];

/** Native names, shown the same in every language. */
export const LANGUAGE_NAMES = Object.fromEntries(
  Object.entries(file.languages).map(([code, language]) => [code, language.name]),
) as Record<Language, string>;

export type Translate = (key: TextKey, values?: Record<string, string | number>) => string;

/** Looks a text up, in English where the language lacks it, and fills its `{name}` placeholders. */
export function makeTranslate(language: Language): Translate {
  return (key, values) => {
    const text = TEXTS[`app.${key}`];
    const value = text?.[language] || text?.en || key;
    return value.replace(/\{(\w+)\}/g, (match, name: string) => (name in (values ?? {}) ? String(values![name]) : match));
  };
}

export const I18nContext = createContext<{ language: Language; t: Translate }>({
  language: 'en',
  t: makeTranslate('en'),
});

export const useT = () => useContext(I18nContext);
