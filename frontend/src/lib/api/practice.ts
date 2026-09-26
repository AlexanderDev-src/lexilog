import type { PracticeInput, PracticeSession } from '../types';
import { http, query } from './client';

export const practiceApi = {
  list: (limit = 50) => http.get<PracticeSession[]>(`/practice${query({ limit })}`),
  create: (input: PracticeInput) => http.post<PracticeSession>('/practice', input),
  update: (id: number, input: PracticeInput) => http.put<PracticeSession>(`/practice/${id}`, input),
  remove: (id: number) => http.del(`/practice/${id}`),
};
