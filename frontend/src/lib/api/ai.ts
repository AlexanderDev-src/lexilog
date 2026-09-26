import type { AiFeedbackRecord, AiStatus } from '../types';
import { http } from './client';

export const aiApi = {
  status: () => http.get<AiStatus>('/ai/status'),
  forPiece: (pieceId: number) => http.get<AiFeedbackRecord[]>(`/pieces/${pieceId}/ai-feedback`),
  /** Sends the version to the model. Takes 10-30 seconds. */
  review: (versionId: number, model?: string) =>
    http.post<AiFeedbackRecord>(`/versions/${versionId}/ai-feedback`, { model }),
};
