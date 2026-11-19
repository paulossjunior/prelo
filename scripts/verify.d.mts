export interface Step {
  name: string;
  cmd: string;
}

export interface StepResult {
  name: string;
  ok: boolean;
  error?: string;
}

export interface SkipFinding {
  file: string;
  line: number;
}

export declare const STEPS: Step[];
export type VerifyMode = "quick" | "full";
export declare function stepsFor(mode: VerifyMode): Step[];
export declare function modeFromArgs(argv: string[]): VerifyMode;
export declare function runSteps(
  steps: Step[],
  run?: (step: Step) => Promise<number>,
): Promise<StepResult[]>;
export declare function exitCode(results: StepResult[]): number;
export declare function summarize(results: StepResult[]): string;
export declare function findUnjustifiedSkips(file: string, text: string): SkipFinding[];
export declare function scanSkips(root: string): SkipFinding[];
export declare function isMain(moduleUrl: string, argv1: string | undefined): boolean;
