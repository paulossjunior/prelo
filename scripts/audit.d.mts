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

export interface FontLicense {
  file: string;
  family: string;
  license: string;
  copyright: string;
}

export interface EmbeddedData {
  name: string;
  crate: string;
  path: string;
  licenses: string[];
}

export interface DenyLicenseException {
  crate?: string;
  name?: string;
  allow?: string[];
}

export declare function parseFontLicenses(tomlText: string): FontLicense[];
export declare function fontLicenseViolations(
  files: string[],
  entries: FontLicense[],
  allow: string[],
): string[];
export declare function parseEmbeddedData(tomlText: string): EmbeddedData[];
export declare function embeddedDataViolations(
  data: EmbeddedData[],
  allow: string[],
  denyExceptions: DenyLicenseException[],
): string[];
