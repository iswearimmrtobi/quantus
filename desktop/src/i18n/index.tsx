import { createContext, useContext } from 'react';
import type { Language } from '../api';
import { de } from './de';
import { en, type TextKey } from './en';

const DICTIONARIES: Record<Language, Record<TextKey, string>> = { en, de };

/** Native names, shown the same in every language. */
export const LANGUAGE_NAMES: Record<Language, string> = { en: 'English', de: 'Deutsch' };

export type Translate = (key: TextKey, values?: Record<string, string | number>) => string;

/** Looks a text up and fills its `{name}` placeholders. */
export function makeTranslate(language: Language): Translate {
  const dictionary = DICTIONARIES[language];
  return (key, values) =>
    dictionary[key].replace(/\{(\w+)\}/g, (match, name: string) => (name in (values ?? {}) ? String(values![name]) : match));
}

export const I18nContext = createContext<{ language: Language; t: Translate }>({
  language: 'en',
  t: makeTranslate('en'),
});

export const useT = () => useContext(I18nContext);
