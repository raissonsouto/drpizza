# Documentação - Dr. Pizza CLI

Referência completa de todos os comandos, flags e opções da CLI.

## Flag global

| Flag | Descrição |
|------|-----------|
| `-u <ID>`, `--unidade <ID>` | Define a unidade a ser usada no comando |

Esta flag pode ser combinada com qualquer subcomando.

---

## Comandos

### `pedir`

Inicia o assistente interativo de pedido. Este é o comando padrão quando nenhum subcomando é informado.

**Flags:**

| Flag | Descrição |
|------|-----------|
| `-s`, `--stateless` | Modo anônimo: ignora dados salvos em `~/.drpizza` |
| `--no-cache` | Ignora cache e força busca atualizada do cardápio |

**Exemplos:**

```bash
# Iniciar pedido normalmente
drpizza pedir

# Pedido com unidade específica
drpizza pedir -u 5

# Pedido em modo anônimo (sem dados salvos)
drpizza pedir -s

# Forçar atualização do cardápio
drpizza pedir --no-cache
```

---

### `menu`

Exibe o cardápio completo com preços e bordas.

**Flags:**

| Flag | Descrição |
|------|-----------|
| `--no-pagination` | Exibe o cardápio completo sem paginação |
| `--no-cache` | Ignora cache e força busca atualizada do cardápio |

**Exemplos:**

```bash
# Ver cardápio com paginação
drpizza menu

# Ver cardápio completo de uma vez
drpizza menu --no-pagination

# Ver cardápio atualizado de uma unidade específica
drpizza menu -u 3 --no-cache
```

---

### `unidades`

Lista as unidades disponíveis. Por padrão, filtra pelo bairro do endereço padrão.

**Flags:**

| Flag | Descrição |
|------|-----------|
| `-a`, `--all` | Mostra todas as unidades (sem filtro por bairro) |
| `--detalhes` | Exibe visão detalhada das unidades |
| `-d <ID>`, `--default <ID>` | Define uma unidade como padrão para o endereço atual |
| `--no-default` | Remove a unidade padrão do endereço atual |

**Exemplos:**

```bash
# Listar unidades próximas
drpizza unidades

# Listar todas as unidades
drpizza unidades -a

# Ver detalhes de todas as unidades
drpizza unidades -a --detalhes

# Definir unidade 3 como padrão
drpizza unidades -d 3

# Remover unidade padrão
drpizza unidades --no-default
```

---

### `pedido`

Mostra o status do último pedido realizado.

**Exemplos:**

```bash
# Ver último pedido
drpizza pedido

# Ver último pedido de uma unidade específica
drpizza pedido -u 5
```

---

### `pedidos`

Lista o histórico completo de pedidos.

**Exemplos:**

```bash
# Ver histórico de pedidos
drpizza pedidos
```

---

### `perfil`

Visualiza ou edita o perfil local (nome, telefone, client ID).

**Flags:**

| Flag | Descrição |
|------|-----------|
| `-e`, `--edit` | Edita o perfil interativamente |

**Exemplos:**

```bash
# Ver perfil
drpizza perfil

# Editar perfil
drpizza perfil --edit
```

---

### `enderecos`

Gerencia endereços de entrega salvos localmente.

**Flags:**

| Flag | Descrição |
|------|-----------|
| `-s`, `--show` | Exibe endereços e sai (sem menu interativo) |
| `-d <N>`, `--default <N>` | Define endereço padrão pelo índice (começa em 1) |
| `-r <N>`, `--remove <N>` | Remove endereço pelo índice (começa em 1) |
| `-a`, `--add` | Adiciona novo endereço diretamente |

**Exemplos:**

```bash
# Gerenciar endereços interativamente
drpizza enderecos

# Apenas listar endereços
drpizza enderecos -s

# Definir o segundo endereço como padrão
drpizza enderecos -d 2

# Remover o terceiro endereço
drpizza enderecos -r 3

# Adicionar novo endereço
drpizza enderecos -a
```

---

## Arquivos locais

| Arquivo | Descrição |
|---------|-----------|
| `~/.drpizza` | Perfil do usuário (nome, telefone, client ID, endereços, preferências) |
| `~/.drpizza_menu_cache.json` | Cache do cardápio (válido por 30 minutos) |

O modo `--stateless` ignora completamente esses arquivos, ideal para máquinas compartilhadas ou testes.
