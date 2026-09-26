// Finds the deck's words in a piece of writing, including basic inflections:
// a card "mitigate" also matches "mitigates", "mitigated" and "mitigating".
// Common irregular words use their real forms instead (rise -> rose, risen),
// so a learner's "rised" is never underlined as a correct deck word.

import type { DeckWord } from './types';

/** A deck word found in the text: `text.slice(start, end)` is the match. */
export interface DeckMatch {
  cardId: number;
  /** The card's word as saved in the deck. */
  word: string;
  start: number;
  end: number;
}

/** A word: letters or digits, joined by inner apostrophes or hyphens ("well-being", "don't"). */
const WORD = /[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu;

interface Token {
  text: string; // lower case, straight apostrophes
  start: number;
  end: number;
}

function tokenize(text: string): Token[] {
  return [...text.matchAll(WORD)].map((m) => ({
    text: m[0].toLowerCase().replaceAll('’', "'"),
    start: m.index,
    end: m.index + m[0].length,
  }));
}

/**
 * Irregular words common in IELTS writing, as "base form form...". Their
 * regular "-ed" forms are dropped: they are mistakes, not deck words.
 */
const IRREGULAR = new Map(
  `arise arose arisen|become became|begin began begun|bring brought|build built|buy bought|catch caught|
  choose chose chosen|come came|cut|deal dealt|do did done does|draw drew drawn|drive drove driven|eat ate eaten|
  fall fell fallen|feel felt|find found|forecast|forget forgot forgotten|get got gotten|give gave given|
  go went gone goes|grow grew grown|have had has|hold held|keep kept|know knew known|lead led|leave left|lend lent|
  lose lost|make made|mean meant|meet met|overtake overtook overtaken|pay paid|put|quit|read|rise rose risen|
  run ran|say said|see saw seen|seek sought|sell sold|send sent|set|shrink shrank shrunk|speak spoke spoken|
  spend spent|spread|stand stood|strike struck|take took taken|teach taught|tell told|think thought|
  throw threw thrown|undergo underwent undergone|understand understood|undertake undertook undertaken|win won|
  withdraw withdrew withdrawn|write wrote written|analysis analyses|basis bases|child children|crisis crises|
  criterion criteria|curriculum curricula|datum data|foot feet|hypothesis hypotheses|man men|medium media|
  person people|phenomenon phenomena|thesis theses|tooth teeth|woman women`
    .split('|')
    .map((entry) => entry.trim().split(/\s+/))
    .map(([base, ...forms]) => [base, forms] as const),
);

/**
 * Inflections of one word. The regular rules generate a few forms that
 * don't exist ("mitigateing"); that is harmless, nobody writes them.
 */
export function inflections(word: string): string[] {
  const w = word.toLowerCase();
  const forms = new Set([w]);
  const add = (...more: string[]) => more.forEach((f) => forms.add(f));

  if (/[^aeiou]y$/.test(w)) {
    // study -> studies, studied, studying
    const stem = w.slice(0, -1);
    add(`${stem}ies`, `${stem}ied`, `${w}ing`);
  } else if (w.endsWith('ie')) {
    // die -> dies, died, dying
    add(`${w}s`, `${w}d`, `${w.slice(0, -2)}ying`);
  } else if (w.endsWith('e')) {
    // mitigate -> mitigates, mitigated, mitigating; agree -> agreeing
    add(`${w}s`, `${w}d`, `${w.slice(0, -1)}ing`, `${w}ing`);
  } else if (/(s|x|z|ch|sh|o)$/.test(w)) {
    // focus -> focuses, focused; approach -> approaches
    add(`${w}es`, `${w}ed`, `${w}ing`, `${w}s`);
  } else {
    add(`${w}s`, `${w}ed`, `${w}ing`);
  }
  // Short final syllable: commit -> committed, committing; plan -> planned.
  if (/[^aeiou][aeiou][bdgklmnprt]$/.test(w)) {
    const last = w.at(-1);
    add(`${w}${last}ed`, `${w}${last}ing`);
  }
  const irregular = IRREGULAR.get(w);
  if (irregular) {
    for (const form of forms) if (form !== w && form.endsWith('ed')) forms.delete(form);
    add(...irregular);
  }
  return [...forms];
}

/**
 * The phrases a card can match. "mitigate (v)" -> [["mitigate"]];
 * "affect / effect" -> [["affect"], ["effect"]];
 * "play a key role" -> [["play", "a", "key", "role"]].
 */
function entryPhrases(word: string): string[][] {
  return word
    .replace(/\([^)]*\)/g, ' ') // drop notes in brackets
    .split(/[/;,]/)
    .map((part) => tokenize(part).map((t) => t.text))
    .filter((words) => words.length > 0);
}

interface Phrase {
  cardId: number;
  word: string;
  /** For each word of the phrase, every form it may take. */
  forms: Set<string>[];
}

export type Matcher = (text: string) => DeckMatch[];

/** Prepares the deck once; the returned function is fast enough to run on every keystroke. */
export function buildMatcher(deck: DeckWord[]): Matcher {
  // Phrases indexed by the forms of their first word, longest phrase first.
  const byFirst = new Map<string, Phrase[]>();
  for (const card of deck) {
    for (const words of entryPhrases(card.word)) {
      const phrase: Phrase = { cardId: card.id, word: card.word, forms: words.map((w) => new Set(inflections(w))) };
      for (const form of phrase.forms[0]) {
        const list = byFirst.get(form) ?? [];
        list.push(phrase);
        byFirst.set(form, list);
      }
    }
  }
  for (const list of byFirst.values()) list.sort((a, b) => b.forms.length - a.forms.length);

  return (text) => {
    const tokens = tokenize(text);
    const matches: DeckMatch[] = [];
    let i = 0;
    while (i < tokens.length) {
      const phrase = byFirst.get(tokens[i].text)?.find((p) => fits(p, tokens, i, text));
      if (phrase) {
        const last = tokens[i + phrase.forms.length - 1];
        matches.push({ cardId: phrase.cardId, word: phrase.word, start: tokens[i].start, end: last.end });
        i += phrase.forms.length;
      } else {
        i += 1;
      }
    }
    return matches;
  };
}

/** Does the phrase match the tokens starting at `i`, separated only by spaces? */
function fits(phrase: Phrase, tokens: Token[], i: number, text: string): boolean {
  if (i + phrase.forms.length > tokens.length) return false;
  for (let k = 1; k < phrase.forms.length; k++) {
    const token = tokens[i + k];
    const gap = text.slice(tokens[i + k - 1].end, token.start);
    if (!/^\s+$/.test(gap) || !phrase.forms[k].has(token.text)) return false;
  }
  return true;
}

/**
 * The sentence around `text[start..end]`, for a card's example. Stops at
 * . ! ? followed by a space, and at line breaks.
 */
export function sentenceAround(text: string, start: number, end: number): string {
  let from = start;
  while (from > 0 && !/[.!?]\s$|\n$/.test(text.slice(Math.max(0, from - 2), from))) from--;
  let to = end;
  while (to < text.length && !/^[.!?](\s|$)|^\n/.test(text.slice(to, to + 2))) to++;
  if (to < text.length && text[to] !== '\n') to++; // keep the full stop
  const sentence = text.slice(from, to).replace(/\s+/g, ' ').trim();
  return sentence.length > 300 ? `${sentence.slice(0, 297)}…` : sentence;
}
