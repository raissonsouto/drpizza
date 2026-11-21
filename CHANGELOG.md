# Changelog

## [0.2.0] - 2026-11-21

### Adicionado
- Notificações no desktop a cada mudança de status do pedido (ex.: "Saiu para entrega!"), ativadas por padrão após o `pedir`; acompanhamento em segundo plano a cada 30s, por até 4h (`notify-send` no Linux, `osascript` no macOS).
- Modo silencioso no perfil: `drpizza perfil --edit` (campo `notificacoes` em `~/.drpizza`).
- `drpizza status --watch`: acompanha o pedido no terminal e notifica a cada mudança.
- Prazo de pagamento do PIX (`⏳ Pague até HH:MM`) no fim do `pedir` e em `status`/`pedidos` enquanto pendente.

### Alterado
- Adicionais escolhidos numa única linha (`1 3x2`, ENTER pula) em vez de uma pergunta por item.
- Preços do cardápio iguais ao app web: "a partir de R$ X" para itens com preço pelos sabores, soma das etapas em combos e preço original "(de R$ X)" em promoções.
- `pedir` não usa mais cache desatualizado se a API do cardápio falhar; o pedido é interrompido.

### Corrigido
- `pedir`: pedidos recusados com `O preço do produto '...' mudou` (HTTP 500).
  - Preços promocionais só valem dentro do horário da promoção (`promotional_price_schedules`).
  - Categorias/itens com horário restrito (ex.: Happy Hour) só aparecem dentro da janela (`allowed_times`).
  - Preço dos sabores segue `price_calculation_type` (`SUM`, `MEAN`, `MIN`, `MAX`); antes os adicionais de sabor eram ignorados em itens com preço base.
  - Payload dos subitens igual ao app web: `unit_price` = preço base, preço de sabores/bordas rateado por grupo e `add_on_selected_subitems_count`.
  - Preços são revalidados ao confirmar; itens fora do horário são removidos ou têm o preço atualizado.
- Cardápio falhava ao parsear campos de lista enviados como `""` pela API.
- Itens com preço pelos sabores e combos apareciam como `R$ 0.00` no cardápio.

## [0.1.0]
- Versão inicial.
