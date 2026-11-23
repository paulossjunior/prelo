# D-04 — Conexão P2P com iroh, sem servidor obrigatório

- **Status:** aceita · novembro de 2026
- **Contexto:** artigos inéditos não devem ficar em servidor de terceiros (RNF-02), e o grupo não deve ter custo de infraestrutura. O Yjs já resolve conflitos localmente; falta só os computadores se encontrarem.
- **Decisão:** mDNS na rede local e iroh pela internet, conectando pela chave pública de cada pessoa. Relay iroh só para conectar quando o NAT bloqueia, com tráfego criptografado de ponta a ponta.
- **Alternativas consideradas:**
  - y-webrtc: pronto para Yjs, mas precisa de servidor de sinalização e de TURN em redes fechadas.
  - libp2p: mesma função do iroh, mais complexo.
- **Consequências:**
  - (+) Roda no mesmo núcleo Rust, ao lado do yrs; não depende de endereço IP.
  - (−) Redes que bloqueiam UDP forçam todo o tráfego pelo relay. Falta decidir entre relays públicos do iroh e relay próprio do grupo.

---

Índice de decisões: [adrs.md](../adrs.md)
