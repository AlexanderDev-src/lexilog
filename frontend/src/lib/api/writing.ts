import type { Piece, PieceSummary, Version, WritingKind } from '../types';
import { http } from './client';

export interface PieceUpdate {
  kind: WritingKind;
  prompt: string;
  written_on: string;
}

export interface VersionUpdate {
  body: string;
  seconds_spent?: number;
  feedback?: string;
}

export const writingApi = {
  list: () => http.get<PieceSummary[]>('/pieces'),
  get: (id: number) => http.get<Piece>(`/pieces/${id}`),
  create: (kind: WritingKind, prompt = '') => http.post<Piece>('/pieces', { kind, prompt }),
  update: (id: number, update: PieceUpdate) => http.put<Piece>(`/pieces/${id}`, update),
  remove: (id: number) => http.del(`/pieces/${id}`),

  addVersion: (pieceId: number) => http.post<Version>(`/pieces/${pieceId}/versions`),
  updateVersion: (id: number, update: VersionUpdate) => http.put<Version>(`/versions/${id}`, update),
  removeVersion: (id: number) => http.del(`/versions/${id}`),

  /** Attaches (or replaces) the chart image; returns the updated piece. */
  putImage: (pieceId: number, image: Blob) => http.put<Piece>(`/pieces/${pieceId}/image`, image),
  removeImage: (pieceId: number) => http.del(`/pieces/${pieceId}/image`),
  /** `?v=` changes when the image is replaced, so the browser never shows an old one. */
  imageUrl: (pieceId: number, uploadedAt: string) =>
    `/api/pieces/${pieceId}/image?v=${encodeURIComponent(uploadedAt)}`,
};
