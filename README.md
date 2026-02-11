
# 🍕 Dr. Pizza CLI

> **"Aqui é RECHEIO com PIZZA!"**

CLI para fazer pedidos no Dr. Pizza diretamente pelo terminal. Escolha entre **Delivery** ou **Retirada**, consulte o cardápio atualizado e acompanhe seus pedidos.

## Instalação

```bash
curl -L -o drpizza https://github.com/raissonsouto/drpizza-cli/releases/latest/download/drpizza_linux_amd64 && chmod +x drpizza && sudo mv drpizza /usr/local/bin/
```

Agora, basta digitar `drpizza` em qualquer lugar do terminal para pedir sua pizza!

## Comandos

| Comando | Descrição |
|---------|-----------|
| `drpizza pedir` | Inicia o assistente de pedido interativo (padrão) |
| `drpizza menu` | Exibe o cardápio completo com preços |
| `drpizza unidades` | Lista unidades disponíveis |
| `drpizza pedido` | Mostra status do último pedido |
| `drpizza pedidos` | Lista histórico de pedidos |
| `drpizza perfil` | Visualiza o perfil local (nome, telefone) |
| `drpizza perfil --edit` | Edita o perfil interativamente |
| `drpizza enderecos` | Gerencia endereços de entrega |

## Flags Globais

| Flag | Descrição |
|------|-----------|
| `-u <ID>`, `--unidade <ID>` | Define a unidade antecipadamente |
| `-s`, `--stateless` | Modo anônimo: ignora dados salvos em `~/.drpizza` |
| `--no-cache` | Ignora cache e força busca atualizada do cardápio |

## Flags de Comando

| Flag | Comando | Descrição |
|------|---------|-----------|
| `--no-pagination` | `menu` | Exibe o cardápio completo sem paginação |
| `--edit` | `perfil` | Edita o perfil interativamente |

## Modo Anônimo (`--stateless`)

Quando ativado com `-s` ou `--stateless`, o sistema opera de forma completamente stateless:

- Nenhum arquivo é lido ou gravado em `~/.drpizza`
- Todo o fluxo é executado como se fosse a primeira execução
- Dados necessários (nome, telefone, endereço) são solicitados durante o pedido
- Ideal para uso em máquinas compartilhadas ou testes

## Perfil e Endereços

O perfil local é armazenado em `~/.drpizza` e inclui:

- **Nome** e **telefone** do cliente (gerenciados via `drpizza perfil`)
- **Client ID** para consulta de pedidos
- **Endereços** de entrega salvos (gerenciados via `drpizza enderecos`)
- **Unidade padrão** e **endereço padrão**

O endereço padrão é usado para sugerir automaticamente a unidade mais adequada com base no bairro.
