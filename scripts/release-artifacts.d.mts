export interface InstallerMatch {
  found: string[];
  missing: string[];
  extra: string[];
}

export declare function expectedInstallers(version: string): string[];
export declare function matchInstallers(files: string[], version: string): InstallerMatch;

export interface SumEntry {
  file: string;
  sha256: string;
}

export declare const SUMS_FILE: "SHA256SUMS.txt";
export declare function sha256File(path: string): Promise<string>;
export declare function formatSums(entries: SumEntry[]): string;
export declare function verifySums(text: string, dir: string): Promise<string[]>;
export declare function containsSelfTestMarker(buffer: Uint8Array): boolean;
export declare function fileHasSelfTestMarker(path: string, chunkSize?: number): Promise<boolean>;
