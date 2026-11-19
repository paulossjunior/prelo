export interface DeclaredVersion {
  file: string;
  version: string;
}

export interface ReleaseInput {
  tag: string;
  root: string;
  commit: string;
}

export declare function parseTag(tag: string): { version: string };
export declare function isOnMain(commit: string, cwd: string): boolean;
export declare function readVersions(root: string): DeclaredVersion[];
export declare function versionMismatches(version: string, versions: DeclaredVersion[]): DeclaredVersion[];
export declare function findLicense(root: string): string | null;
export declare function checkRelease(input: ReleaseInput): string[];
