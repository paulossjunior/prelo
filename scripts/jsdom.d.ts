// Tipos mínimos do jsdom (já é devDependency, sem @types/jsdom) usados por scripts/site.test.ts.
declare module "jsdom" {
  export class JSDOM {
    constructor(html?: string);
    readonly window: Window & typeof globalThis;
  }
}
