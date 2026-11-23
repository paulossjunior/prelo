export const KILL_AFTER_MS: number;
export const MAX_LOSS_MS: number;
export interface Mark {
  n: number;
  at: number;
}
export function markText(n: number): string;
export function parseMark(line: string): number | null;
export function parseReopened(line: string): string | null;
export function checkReopened(args: {
  marks: Mark[];
  killedAt: number;
  original: string;
  reopened: string | null;
  maxLossMs?: number;
}): string[];
