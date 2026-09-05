# Z3S — High-Performance S3-Compatible Object Storage

O **Z3S** é um sistema de armazenamento de objetos distribuído de alto desempenho compatível com a API AWS S3, desenvolvido em Rust do zero para máxima eficiência de I/O, segurança e resiliência em escala de petabytes.

## 📖 Documentação & Especificações

- 🗺️ **[ROADMAP.md](file:///home/moskvat/Developer/z3s/ROADMAP.md)**: Blueprint arquitetural detalhado, stack tecnológica de alta performance e roadmap completo em 7 fases de engenharia (Fase 0 a Fase 6).
- 🏗️ **[ARCHITECTURE.md](file:///home/moskvat/Developer/z3s/ARCHITECTURE.md)**: Estrutura modular de diretórios (Cargo Workspace multi-crate), responsabilidades dos componentes e fluxos de dados de ponta a ponta (`PUT`, `GET`, etc.).

## 🚀 Princípios Fundamentais

- **Desacoplamento de Planos:** Separação estrita entre *Data Path* (shards de conteúdo) e *Metadata Path* (índice, catálogo e consenso).
- **Sem Garbage Collection:** Desenvolvido em Rust para latências p99 ultra-baixas sem pausas de GC.
- **I/O Moderno:** `io_uring` e Direct I/O (`O_DIRECT`) para contornar gargalos do kernel do Linux.
- **Tolerância a Falhas:** Erasure Coding acelerado por SIMD/AVX (Reed-Solomon) e proteção contínua contra corrupção silenciosa (*Bitrot Scrubbing*).
- **Consistência Estrita:** Garantia de consistência forte (*Strong Read-After-Write*) com modelo *Witness*.
