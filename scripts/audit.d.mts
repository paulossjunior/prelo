export interface VulnException {
  id?: string;
  ecosystem?: string;
  reason?: string;
  review_by?: string;
}

export interface PackageLicense {
  name: string;
  licenses?: string;
}

export declare function parseAllowlist(text: string): string[];
export declare function denyAllowList(tomlText: string): string[];
export declare function denyAdvisoryIgnores(tomlText: string): string[];
export declare function parseExceptions(tomlText: string): VulnException[];
export declare function validateExceptions(list: VulnException[], today: Date): string[];
export declare function exceptionIds(list: VulnException[], ecosystem: "cargo" | "npm"): string[];
export declare function licenseViolations(
  packages: PackageLicense[],
  allow: string[],
): PackageLicense[];
