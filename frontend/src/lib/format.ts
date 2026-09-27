import type { PartOfSpeech, Skill, WritingKind } from './types';

/** Same rule as the backend: anything between whitespace is one word. */
export function countWords(text: string): number {
  return text.split(/\s+/).filter(Boolean).length;
}

/** 600 -> "10m", 86400 -> "1d", 3888000 -> "1.5mo" */
export function formatInterval(seconds: number): string {
  const minutes = seconds / 60;
  const days = seconds / 86_400;
  if (minutes < 60) return `${Math.round(minutes)}m`;
  if (days < 1) return `${Math.round(minutes / 60)}h`;
  if (days < 30) return `${Math.round(days)}d`;
  if (days < 365) return `${(days / 30).toFixed(1).replace(/\.0$/, '')}mo`;
  return `${(days / 365).toFixed(1).replace(/\.0$/, '')}y`;
}

/** 125 -> "02:05", 3725 -> "1:02:05" */
export function formatClock(totalSeconds: number): string {
  const s = Math.abs(Math.floor(totalSeconds));
  const hours = Math.floor(s / 3600);
  const mm = String(Math.floor((s % 3600) / 60)).padStart(2, '0');
  const ss = String(s % 60).padStart(2, '0');
  return hours ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
}

/** "due now", "due in 3d", "due 2d ago" */
export function formatDue(dueIso: string, now = Date.now()): string {
  const diff = (new Date(dueIso).getTime() - now) / 1000;
  if (Math.abs(diff) < 60) return 'due now';
  return diff > 0 ? `due in ${formatInterval(diff)}` : `due ${formatInterval(-diff)} ago`;
}

export function formatDate(isoDate: string): string {
  return new Date(`${isoDate}T00:00:00`).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
}

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}

export interface WritingPreset {
  label: string;
  /** Timer length; null = stopwatch that just counts up. */
  minutes: number | null;
  /** IELTS minimum; null = no target. */
  minWords: number | null;
}

export const PRESETS: Record<WritingKind, WritingPreset> = {
  task1: { label: 'Task 1', minutes: 20, minWords: 150 },
  task2: { label: 'Task 2', minutes: 40, minWords: 250 },
  paragraph: { label: 'Paragraph', minutes: null, minWords: null },
};

/** Short label shown after a word ("n."), and the full name for the picker. */
export const PARTS_OF_SPEECH: { value: PartOfSpeech; short: string; name: string }[] = [
  { value: 'n', short: 'n.', name: 'noun' },
  { value: 'v', short: 'v.', name: 'verb' },
  { value: 'adj', short: 'adj.', name: 'adjective' },
  { value: 'adv', short: 'adv.', name: 'adverb' },
  { value: 'prep', short: 'prep.', name: 'preposition' },
  { value: 'conj', short: 'conj.', name: 'conjunction' },
  { value: 'phrv', short: 'phr. v.', name: 'phrasal verb' },
  { value: 'phrase', short: 'phrase', name: 'phrase' },
  { value: 'idiom', short: 'idiom', name: 'idiom' },
];

/** 'adj' -> "adj.", '' -> "" */
export function posLabel(pos: PartOfSpeech | ''): string {
  return PARTS_OF_SPEECH.find((p) => p.value === pos)?.short ?? '';
}

/** 'adj' -> "adjective", '' -> "" */
export function posName(pos: PartOfSpeech | ''): string {
  return PARTS_OF_SPEECH.find((p) => p.value === pos)?.name ?? '';
}

export const SKILLS: { value: Skill; label: string }[] = [
  { value: 'listening', label: 'Listening' },
  { value: 'reading', label: 'Reading' },
  { value: 'speaking', label: 'Speaking' },
  { value: 'other', label: 'Other' },
];

/** 1234 -> "1.2k", 200000 -> "200k" */
export function formatTokens(n: number): string {
  if (n < 1000) return String(n);
  const k = n / 1000;
  return `${k < 10 ? k.toFixed(1).replace(/\.0$/, '') : Math.round(k)}k`;
}

/** "2026-09" -> "Sep 2026", "2026-W38" -> "W38 2026" */
export function formatPeriod(period: string): string {
  const week = period.match(/^(\d{4})-W(\d{2})$/);
  if (week) return `W${week[2]} ${week[1]}`;
  const [y, m] = period.split('-').map(Number);
  return new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString(undefined, {
    month: 'short',
    year: 'numeric',
    timeZone: 'UTC',
  });
}
