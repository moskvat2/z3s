# Z3S Web Console — Frontend

A interface gráfica oficial do **Z3S S3-Compatible Object Storage Server**.

## 🎨 Características:
- **Pilha:** React 18 + Tailwind CSS + SigV4 Web Crypto Client.
- **Autenticação:** Assinatura criptográfica AWS SigV4 no navegador via Web Crypto API.
- **Gerenciamento:**
  - Visualização e criação de buckets.
  - Navegador de arquivos e pastas virtuais.
  - Alternador de tema Dark / Light Mode.
- **Entrega em Produção:** Servido diretamente na rota `/console/` do servidor `z3s-server`.
