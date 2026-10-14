// Plural forms for the locale files, chosen by the CLDR rules built into the WebView (Intl.PluralRules).
//
// Categories per language (integers): en/de/it/es: one, other · fr: one (0 and 1), many (1 000 000…), other ·
// pl/uk/ru: one, few, many · zh/ja: other only. `other` is the fallback for any category a call leaves out.

export type PluralForms = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };

/** `(n, forms)` → the form for `n` in `locale`, e.g. `plural(3, { one: "link", other: "links" })`. */
export function pluralizer(locale: string): (n: number, forms: PluralForms) => string {
  const rules = new Intl.PluralRules(locale);
  return (n, forms) => forms[rules.select(n)] ?? forms.other;
}
