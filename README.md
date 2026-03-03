# Dr. Pizza CLI

CLI para consultar cardápio e fazer pedidos da Dr. Pizza direto do terminal.

## Instalação

### Script (rápido)

```bash
curl -fsSL https://raw.githubusercontent.com/raissonsouto/drpizza/main/install.sh | bash
```

### Manual

```bash
git clone https://github.com/raissonsouto/drpizza.git
cd drpizza
cargo build --release
```

Binário gerado em `target/release/drpizza`.

## Comandos principais

- `pedir`: fluxo completo de pedido (carrinho, endereço, pagamento, confirmação)
- `menu`: navegação no cardápio
- `unidades`: lista unidades e detalhes
- `status`: mostra último pedido (`pedido` funciona como alias)
- `pedidos`: mostra histórico
- `perfil`: visualiza/edita perfil local
- `enderecos`: gerencia endereços salvos

## Documentação completa

- Referência de comandos: [DOCS.md](DOCS.md)
- Guia para contribuição: [CONTRIBUTING.md](CONTRIBUTING.md)
