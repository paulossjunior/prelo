export interface InstallerMatch {
  found: string[];
  missing: string[];
  extra: string[];
}

export declare function expectedInstallers(version: string): string[];
export declare function matchInstallers(files: string[], version: string): InstallerMatch;
