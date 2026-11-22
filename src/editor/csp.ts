// Nonce de estilo para o CodeMirror sob a CSP `style-src 'self'` (T037; contracts/ipc.md,
// "Permissões da janela").
//
// O CodeMirror (via `style-mod`) põe o tema base num <style> criado em tempo de execução; a CSP
// bloqueia esse <style> e o editor fica sem layout. Em vez de liberar `'unsafe-inline'`, usamos
// o nonce que o próprio Tauri gera: ao servir index.html ele troca o marcador do <style> vazio
// por um número aleatório novo a cada carga e acrescenta `'nonce-…'` ao `style-src` da CSP
// servida (o tauri.conf.json não muda). Só um <style> com esse nonce é aceito, e só script da
// própria origem (`script-src` herda `'self'`) consegue lê-lo.

/** Marcador que o Tauri põe no atributo `nonce` dos <style> de index.html e troca ao servir. */
export const STYLE_NONCE_TOKEN = "__TAURI_STYLE_NONCE__";

/** Nonce de estilo da carga atual da página, se o Tauri pôs um no <style data-csp-nonce>. */
export function styleNonce(doc: Document = document): string | undefined {
  const el = doc.querySelector<HTMLStyleElement>("style[data-csp-nonce]");
  // O navegador esconde o atributo depois de carregar; a propriedade `nonce` continua legível.
  const nonce = el?.nonce || el?.getAttribute("nonce") || "";
  return nonce && nonce !== STYLE_NONCE_TOKEN ? nonce : undefined;
}
