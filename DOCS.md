# Documentação - Dr. Pizza CLI

Referência de comandos, flags e comportamento da CLI.

## Flag global

| Flag | Descrição |
|---|---|
| `-u <ID>`, `--unidade <ID>` | Define unidade por **ID da lista** (`0,1,2...`) |

Exemplo:

```bash
drpizza unidades --all
drpizza cardapio -u 0
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

- Adicionais são escolhidos numa única linha pelos números da lista: `1 3x2` adiciona o item 1 e duas unidades do item 3; ENTER pula.
- Categorias/itens com horário restrito (ex.: Happy Hour) só aparecem dentro da janela, e preços promocionais só valem no horário da promoção.
- O preço dos sabores segue a regra do cardápio (`price_calculation_type`): soma (`SUM`), média (`MEAN`), menor (`MIN`) ou maior valor (`MAX`), igual ao app web.
- O `pedir` sempre busca o cardápio atualizado; se a API falhar, o pedido é interrompido em vez de usar o cache (preços desatualizados seriam recusados).
- Ao confirmar, os preços são revalidados: se a janela de um item ou promoção fechou durante o checkout, o item é removido ou o preço atualizado e o resumo é exibido novamente.
- A confirmação final permite editar pagamento, observação, troco, itens e endereço.
- Se o pagamento for PIX, o final do fluxo exibe QR Code no terminal, o código "copia e cola" e o prazo para pagamento (após ele o pedido é cancelado automaticamente).
- Após enviar o pedido, um processo em segundo plano acompanha o status (a cada 30s, por até 4h) e envia uma notificação no desktop a cada mudança (ex.: "Saiu para entrega!"). Usa `notify-send` (Linux) ou `osascript` (macOS). Desative em `drpizza perfil --edit`.
- Pagamentos em cartão de crédito, cartão de débito e dinheiro são realizados na entrega.
- No resumo final:
  - `C` confirma
  - `E` edita opções
  - `X` cancela compra

### `cardapio`

Exibe/navega no cardápio.

Preços seguem o app web: itens cujo preço depende dos sabores aparecem como "a partir de R$ X", combos mostram a soma das etapas e promoções exibem o preço original "(de R$ X)".

Flags:

| Flag | Descrição |
|---|---|
| `--no-pagination` | Exibe tudo de uma vez |
| `--no-cache` | Ignora cache de cardápio |

Exemplos:

```bash
drpizza cardapio
drpizza cardapio --no-pagination
drpizza cardapio -u 0 --no-cache
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

Mostra o último pedido encontrado. Se houver PIX pendente, mostra o prazo de pagamento.

Flags:

| Flag | Descrição |
|---|---|
| `-w`, `--watch` | Acompanha o pedido no terminal e notifica a cada mudança de status (Ctrl+C para sair) |

Exemplos:

```bash
drpizza status
drpizza status --watch
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

### `cancelar`

Cancela um pedido pendente.

Flags:

| Flag | Descrição |
|---|---|
| `-i <IDX>`, `--indice <IDX>` | Cancela diretamente pelo índice da lista de pendentes |
| `-m <TEXTO>`, `--motivo <TEXTO>` | Motivo opcional do cancelamento |
| `-y`, `--yes` | Não pede confirmação |

Exemplos:

```bash
drpizza cancelar
drpizza cancelar -i 0
drpizza cancelar -i 0 -m "Mudança de planos" -y
```

### `pontos`

Mostra os pontos acumulados com base no histórico de pedidos e lista benefícios de fidelidade.

Exemplos:

```bash
drpizza pontos
drpizza pontos -u 0
```

Observações:

- Exibe total de pontos acumulados (`earned_points`) somados a partir dos detalhes dos pedidos.
- Mostra benefícios ativos e se já estão disponíveis ou quantos pontos faltam.

### `perfil`

Mostra/edita perfil local.

Flags:

| Flag | Descrição |
|---|---|
| `-e`, `--edit` | Edita nome, telefone, senha auth e notificações de status (ativadas por padrão) |

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
