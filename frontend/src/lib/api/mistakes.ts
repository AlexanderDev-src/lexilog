import type { MistakeCount, MistakeTagSummary, MistakeTrend, TrendBucket } from '../types';
import { http, query } from './client';

export const mistakesApi = {
  forPiece: (pieceId: number) => http.get<MistakeCount[]>(`/pieces/${pieceId}/mistakes`),
  replace: (pieceId: number, mistakes: MistakeCount[]) =>
    http.put<MistakeCount[]>(`/pieces/${pieceId}/mistakes`, mistakes),
  tags: () => http.get<MistakeTagSummary[]>('/mistake-tags'),
  trend: (bucket: TrendBucket) => http.get<MistakeTrend>(`/mistakes/trend${query({ bucket })}`),
};
