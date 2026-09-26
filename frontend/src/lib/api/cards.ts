import type { Card, CardInput, DeckWord, DueQueue, Rating, TagCount } from '../types';
import { http, query } from './client';

export interface CardSearch {
  q?: string;
  tag?: string;
  limit?: number;
  offset?: number;
}

export const cardsApi = {
  list: (search: CardSearch = {}) => http.get<Card[]>(`/cards${query({ ...search })}`),
  get: (id: number) => http.get<Card>(`/cards/${id}`),
  create: (input: CardInput) => http.post<Card>('/cards', input),
  update: (id: number, input: CardInput) => http.put<Card>(`/cards/${id}`, input),
  remove: (id: number) => http.del(`/cards/${id}`),
  tags: () => http.get<TagCount[]>('/tags'),
  /** Every card's id and word, for underlining deck words in writing. */
  words: () => http.get<DeckWord[]>('/cards/words'),

  due: (limit = 200) => http.get<DueQueue>(`/review/due${query({ limit })}`),
  review: (id: number, rating: Rating, durationMs?: number) =>
    http.post<Card>(`/cards/${id}/review`, { rating, duration_ms: durationMs }),
};
