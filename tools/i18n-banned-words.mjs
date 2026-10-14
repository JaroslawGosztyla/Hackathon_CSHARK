#!/usr/bin/env node
// Checks the reward copy of every UI locale (src/lib/i18n/<lang>.ts) for words of loss, failure, scarcity or hurry
// (ADR-022, spec §2.4): the beta rewards never say a streak was lost, never count down, never rush. The reward copy is
// the top-level `rewards` block plus every key elsewhere whose path holds `rewards` (privacy items, the error area)
// or starts with `rewards.` (`errors["rewards.load"]`), and the step errors `stepsOff` / `tooManySteps`. Run by hand:
//
//   node tools/i18n-banned-words.mjs
//
// Exit code 0 = nothing found, 1 = findings (printed as file:line), 2 = a locale file or its rewards block is missing.
// A few keys may use a word on purpose: the answer "Jeszcze nie" / "Not yet" to the session goal (`goalNotYet`), the
// celebration level "Tylko liczby" / "Numbers only" (`settings.celebrationLevel.numbersOnly`), and the plain "could
// not" of an error message (`errors.*`: a file that cannot be read is no streak lost).
import { readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const I18N = join(ROOT, "src", "lib", "i18n");
const LANGS = ["pl", "en", "de", "fr", "it", "es", "uk", "ru", "zh", "ja"];

/** Whole word (letters on neither side; works for Latin, Cyrillic and accented letters). */
const word = (w) => `(?<![\\p{L}\\p{N}])${w}(?![\\p{L}\\p{N}])`;
/** A stem at the start of a word. */
const stem = (s) => `(?<![\\p{L}\\p{N}])${s}`;

/** `notYet`: "not yet" words (allowed only in `goalNotYet`); `only`: "only" words (allowed in `numbersOnly`). */
const RULES = {
  en: {
    banned: [
      word("lost"),
      word("lose"),
      word("loses"),
      word("losing"),
      word("broke"),
      word("broken"),
      "break your streak",
      word("miss"),
      word("missed"),
      word("fail"),
      word("failed"),
      word("failure"),
      "still not",
      word("hurry"),
      "last chance",
      "at risk",
      "don't let .* down",
      stem("expir"),
    ],
    only: [word("only")],
    notYet: ["not yet"],
  },
  pl: {
    banned: [
      stem("strac"),
      stem("przepad"),
      stem("zerwan"),
      stem("przegap"),
      "nie udało ci się",
      stem("pospiesz"),
      "ostatnia szansa",
      stem("zagrożon"),
    ],
    only: [`${word("tylko")}\\s+(\\d|\\$\\{)`],
    notYet: ["jeszcze nie"],
  },
  de: {
    banned: [
      stem("verlor"),
      stem("verlier"),
      stem("gebrochen"),
      stem("verpasst"),
      stem("versäum"),
      stem("gescheitert"),
      stem("fehlgeschlagen"),
      stem("beeil"),
      "letzte chance",
      stem("gefährdet"),
      "läuft ab",
      stem("abgelaufen"),
    ],
    only: [word("nur")],
    notYet: ["noch nicht"],
  },
  fr: {
    banned: [
      stem("perdu"),
      stem("perdre"),
      stem("cassé"),
      stem("brisé"),
      word("raté"),
      stem("manqué"),
      stem("échou"),
      stem("échec"),
      stem("dépêche"),
      "dernière chance",
      "en danger",
      stem("menacé"),
      stem("expir"),
    ],
    only: [word("seulement")],
    notYet: ["pas encore"],
  },
  it: {
    banned: [
      word("perso"),
      word("persa"),
      word("persi"),
      word("perse"),
      stem("perdere"),
      word("rotto"),
      word("rotta"),
      stem("mancat"),
      stem("fallit"),
      stem("falliment"),
      stem("sbrigat"),
      "ultima occasione",
      "a rischio",
      stem("scad"),
    ],
    only: [word("solo"), word("soltanto")],
    notYet: ["non ancora", "ancora non"],
  },
  es: {
    banned: [
      stem("perdid"),
      stem("perder"),
      word("roto"),
      word("rota"),
      stem("fallad"),
      stem("fallaste"),
      stem("fracas"),
      "date prisa",
      stem("apúrate"),
      "última oportunidad",
      "en riesgo",
      stem("expir"),
      stem("caduc"),
    ],
    only: [word("solo"), word("sólo"), word("solamente")],
    notYet: ["todavía no", "aún no"],
  },
  uk: {
    banned: [
      stem("втрат"),
      stem("втрач"),
      stem("загуб"),
      stem("пропущен"),
      stem("обірв"),
      stem("зламан"),
      stem("провал"),
      "не вдалося",
      stem("поспіш"),
      "останній шанс",
      "під загрозою",
      stem("згорі"),
      stem("згоря"),
      stem("спливає"),
    ],
    only: [word("лише"), word("тільки")],
    notYet: ["ще ні", "ще не"],
  },
  ru: {
    banned: [
      stem("потер"),
      stem("утрат"),
      stem("утрач"),
      stem("пропущен"),
      stem("прерван"),
      stem("сорва"),
      "не удалось",
      stem("провал"),
      stem("поторопи"),
      "последний шанс",
      "под угрозой",
      stem("сгор"),
      stem("истека"),
    ],
    only: [word("только"), word("лишь")],
    notYet: ["еще не", "ещё не", "пока нет"],
  },
  zh: {
    banned: ["失去", "丢失", "中断", "断了", "错过", "失败", "仅剩", "只剩", "快点", "赶紧", "最后机会", "危险", "过期", "到期"],
    only: ["仅"],
    notYet: ["还没", "尚未"],
  },
  ja: {
    banned: ["失う", "失っ", "途切れ", "逃し", "失敗", "残り\\s*(\\d|\\$\\{)", "あと\\s*(\\d|\\$\\{)", "急い", "最後のチャンス", "危機", "ピンチ", "期限切れ"],
    only: ["のみ"],
    notYet: ["まだ"],
  },
};

const ALLOW = {
  notYet: (path) => path.at(-1) === "goalNotYet",
  only: (path) => path.join(".").endsWith("celebrationLevel.numbersOnly"),
};

/** "Could not" in an error message (`errors.*`), e.g. "Не удалось прочитать…": allowed there only. */
const ERROR_COULD_NOT = ["не удалось", "не вдалося"];

/** Steps errors outside the `rewards` block that are reward copy too. */
const STEP_KEYS = new Set(["stepsOff", "tooManySteps"]);

/** A key path of reward copy (see the header). */
function isRewardPath(path) {
  return path.some((key) => key === "rewards" || key.startsWith("rewards.") || STEP_KEYS.has(key));
}

/** Every line of the file with its line number; null without a top-level `rewards: {` block. */
function sourceLines(source) {
  const lines = source.split(/\r?\n/);
  if (!lines.includes("  rewards: {")) return null;
  return lines.map((text, i) => ({ text, line: i + 1 }));
}

/** Tracks the key path of each line (nested object keys by indentation). */
function withPaths(block) {
  /** Open objects: their key and the indentation of the line that opened them. */
  const stack = [];
  let lastKey = "";
  const out = [];
  for (const { text, line } of block) {
    const indent = text.match(/^ */)[0].length;
    if (/^\s*\}/.test(text)) {
      // The closing line of an object has the indentation of the line that opened it.
      while (stack.length > 0 && stack.at(-1).indent >= indent) stack.pop();
      lastKey = "";
      out.push({ text, line, path: stack.map((s) => s.name) });
      continue;
    }
    const key = text.match(/^\s*(?:"([^"]+)"|([A-Za-z_$][\w$]*))\s*:/);
    if (key) lastKey = key[1] ?? key[2];
    out.push({ text, line, path: [...stack.map((s) => s.name), lastKey] });
    if (key && /\{\s*$/.test(text)) {
      stack.push({ indent, name: lastKey });
      lastKey = "";
    }
  }
  return out;
}

/** The string literals of a line (quotes, template literals) without `${…}` code and comments. */
function texts(line) {
  const out = [];
  const re = /"((?:[^"\\]|\\.)*)"|`((?:[^`\\]|\\.)*)`/g;
  let m;
  const code = line.replace(/^\s*\/\/.*$|\/\*.*?\*\//g, "");
  while ((m = re.exec(code)) !== null) out.push((m[1] ?? m[2] ?? "").replace(/\$\{[^}]*\}/g, (x) => (x === "${" ? x : "${}")));
  return out;
}

let findings = 0;
let missing = 0;

for (const lang of LANGS) {
  const file = join(I18N, `${lang}.ts`);
  let source;
  try {
    source = readFileSync(file, "utf8");
  } catch {
    console.error(`${relative(ROOT, file)}: missing`);
    missing++;
    continue;
  }
  const block = sourceLines(source);
  if (!block) {
    console.error(`${relative(ROOT, file)}: no top-level \`rewards\` block`);
    missing++;
    continue;
  }
  const rules = RULES[lang];
  const groups = [
    {
      kind: "banned",
      patterns: rules.banned,
      allowed: (path, pattern) => path[0] === "errors" && ERROR_COULD_NOT.includes(pattern),
    },
    { kind: "only", patterns: rules.only, allowed: ALLOW.only },
    { kind: "not yet", patterns: rules.notYet, allowed: ALLOW.notYet },
  ];
  for (const { text, line, path } of withPaths(block)) {
    if (!isRewardPath(path)) continue;
    for (const value of texts(text)) {
      for (const group of groups) {
        for (const pattern of group.patterns) {
          if (group.allowed(path, pattern)) continue;
          const match = value.match(new RegExp(pattern, "iu"));
          if (!match) continue;
          findings++;
          console.log(`${relative(ROOT, file)}:${line}: [${lang}] ${path.join(".")}: ${group.kind} "${match[0]}"`);
        }
      }
    }
  }
}

if (missing > 0) process.exit(2);
if (findings > 0) {
  console.log(`\n${findings} finding(s): reword them (no loss, failure, scarcity or hurry in the rewards texts).`);
  process.exit(1);
}
console.log(`OK: no banned words in the reward copy of ${LANGS.length} locales.`);
