# Documentação - Dr. Pizza CLI

Referência de comandos, flags e comportamento da CLI.

## Flag global

| Flag | Descrição |
|---|---|
| `-u <ID>`, `--unidade <ID>` | Define unidade por **ID da lista** (`0,1,2...`) |

Exemplo:

```bash
drpizza unidades --all
drpizza menu -u 0
```

## Comandos

### `pedir`

Inicia o assistente interativo de compra.

Flags:

| Flag | Descrição |
|---|---|
| `-s`, `--stateless` | Ignora arquivos locais (`~/.drpizza`, cache) |
| `--no-cache` | Força busca de cardápio sem cache |

Exemplos:

```bash
drpizza pedir
drpizza pedir -u 1
drpizza pedir --stateless
```

Observações:

- A confirmação final permite editar pagamento, observação, troco, itens e endereço.
- No resumo final:
  - `C` confirma
  - `E` edita opções
  - `X` cancela compra

### `menu`

Exibe/navega no cardápio.

Flags:

| Flag | Descrição |
|---|---|
| `--no-pagination` | Exibe tudo de uma vez |
| `--no-cache` | Ignora cache de cardápio |

Exemplos:

```bash
drpizza menu
drpizza menu --no-pagination
drpizza menu -u 0 --no-cache
```

### `unidades`

Lista unidades (por padrão, com filtro pelo bairro do endereço padrão).

Flags:

| Flag | Descrição |
|---|---|
| `-a`, `--all` | Mostra todas as unidades |
| `--detalhes` | Visão detalhada |
| `-d <ID>`, `--default <ID>` | Define unidade padrão para o endereço padrão (ID da lista) |
| `--no-default` | Remove unidade padrão do endereço padrão |

Exemplos:

```bash
drpizza unidades
drpizza unidades --all
drpizza unidades --all --detalhes
drpizza unidades --default 0
```

### `status` (alias: `pedido`)

Mostra o último pedido encontrado.

Exemplos:

```bash
drpizza status
drpizza status -u 0
# compatibilidade
drpizza pedido
```

### `pedidos`

Mostra histórico e permite abrir detalhes por índice da lista.

Exemplos:

```bash
drpizza pedidos
drpizza pedidos -u 0
```

### `perfil`

Mostra/edita perfil local.

Flags:

| Flag | Descrição |
|---|---|
| `-e`, `--edit` | Edita nome, telefone e senha auth |

Exemplos:

```bash
drpizza perfil
drpizza perfil --edit
```

### `enderecos`

Gerencia endereços salvos no perfil local.

Flags:

| Flag | Descrição |
|---|---|
| `-s`, `--show` | Lista endereços e sai |
| `-d <N>`, `--default <N>` | Define endereço padrão (1-based) |
| `-r <N>`, `--remove <N>` | Remove endereço (1-based) |
| `-a`, `--add` | Adiciona endereço |

Exemplos:

```bash
drpizza enderecos
drpizza enderecos --show
drpizza enderecos --default 1
drpizza enderecos --remove 2
drpizza enderecos --add
```

## Arquivos locais

| Arquivo | Finalidade |
|---|---|
| `~/.drpizza` | Perfil local (nome, telefone, client_id, token, endereços, preferências) |
| `~/.drpizza_menu_cache.json` | Cache de cardápio (TTL ~30min) |

No modo `--stateless`, ambos são ignorados.

## Modo debug

Para logs detalhados de API:

```bash
make debug
./target/debug/drpizza pedidos
```

Ou:

```bash
cargo run --features dev -- pedidos
```
