// Shapes of the JSON the backend sends and accepts.
// Timestamps are UTC strings like "2026-09-26T03:15:00Z"; dates are "YYYY-MM-DD".

export interface Card {
  id: number;
  word: string;
  meaning: string;
  example: string;
  source: string;
  tags: string[];
  stability: number | null; // null = new card, never reviewed
  difficulty: number | null;
  due: string;
  last_review: string | null;
  reps: number;
  lapses: number;
  created_at: string;
  updated_at: string;
}

export interface CardInput {
  word: string;
  meaning: string;
  example: string;
  source: string;
  tags: string[];
}

export interface TagCount {
  name: string;
  card_count: number;
}

/** Seconds until the next review for each button. */
export interface IntervalPreview {
  again: number;
  hard: number;
  good: number;
  easy: number;
}

export interface DueCard extends Card {
  preview: IntervalPreview;
}

export interface DueQueue {
  total: number;
  cards: DueCard[];
}

/** 1 = Again, 2 = Hard, 3 = Good, 4 = Easy */
export type Rating = 1 | 2 | 3 | 4;

export type WritingKind = 'task1' | 'task2' | 'paragraph';

export interface Version {
  id: number;
  piece_id: number;
  version_no: number;
  body: string;
  word_count: number;
  seconds_spent: number | null;
  feedback: string;
  created_at: string;
  updated_at: string;
}

export interface Piece {
  id: number;
  kind: WritingKind;
  prompt: string;
  written_on: string;
  created_at: string;
  updated_at: string;
  versions: Version[];
}

export interface PieceSummary {
  id: number;
  kind: WritingKind;
  prompt: string;
  written_on: string;
  version_count: number;
  latest_word_count: number;
  /** Timer seconds on the latest version, null if the timer wasn't used. */
  latest_seconds_spent: number | null;
  updated_at: string;
}

// ---- Phase 2 ----------------------------------------------------------

export interface DayActivity {
  date: string;
  reviews: number;
  /** Writing versions written or edited that day. */
  writing: number;
  practice_minutes: number;
  /** Heatmap shade 0-4. */
  level: number;
}

export interface Dashboard {
  today: string;
  /** First date on the heatmap (a Monday). */
  from: string;
  due_today: number;
  due: { new: number; review: number; again: number };
  /** First card in today's queue. */
  next_card: Card | null;
  latest_piece: PieceSummary | null;
  /** Only days with activity. */
  days: DayActivity[];
  active_days_year: number;
  active_days_month: number;
}

export type Skill = 'listening' | 'reading' | 'speaking' | 'other';

export interface PracticeSession {
  id: number;
  practiced_on: string;
  skill: Skill;
  minutes: number;
  note: string;
  created_at: string;
}

export interface PracticeInput {
  practiced_on?: string;
  skill: Skill;
  minutes: number;
  note: string;
}

export interface MistakeCount {
  tag: string;
  count: number;
}

export interface MistakeTagSummary {
  name: string;
  total: number;
  pieces: number;
}

export type TrendBucket = 'week' | 'month';

export interface TrendPoint {
  period: string;
  count: number;
  per_1000_words: number;
}

export interface TagTrend {
  tag: string;
  total: number;
  points: TrendPoint[];
  direction: 'fading' | 'rising' | 'steady' | 'new';
}

export interface MistakeTrend {
  bucket: TrendBucket;
  periods: { period: string; pieces: number; words: number }[];
  tags: TagTrend[];
}

export interface AiIssue {
  quote: string;
  tag: string;
  hint: string;
}

export interface AiFeedback {
  bands: { task: number; coherence: number; lexical: number; grammar: number; overall: number };
  summary: string;
  issues: AiIssue[];
  questions: string[];
}

export interface AiFeedbackRecord {
  id: number;
  version_id: number;
  model: string;
  feedback: AiFeedback;
  input_tokens: number;
  output_tokens: number;
  created_at: string;
}

export interface ModelQuota {
  id: string;
  daily_limit: number;
  used_today: number;
}

export interface AiStatus {
  enabled: boolean;
  default_model: string | null;
  models: ModelQuota[];
  resets_at: string;
}
