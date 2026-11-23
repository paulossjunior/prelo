// O jsdom não diagrama: `Range` não tem `getClientRects`/`getBoundingClientRect`, e o
// CodeMirror usa os dois para rolar até o cursor (`scrollIntoView`). Para os testes que movem
// o cursor do editor, caixas vazias bastam.
export function stubRangeLayout(): void {
  const proto = Range.prototype as Partial<Range>;
  if (!proto.getClientRects) {
    proto.getClientRects = () => ({ length: 0, item: () => null, [Symbol.iterator]: [][Symbol.iterator] }) as unknown as DOMRectList;
  }
  if (!proto.getBoundingClientRect) {
    proto.getBoundingClientRect = () => new DOMRect(0, 0, 0, 0);
  }
}
