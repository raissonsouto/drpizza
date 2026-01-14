
# 🍕 Dr. Pizza CLI

> **"Aqui é RECHEIO com PIZZA!"**

Chega de ligar ou mandar mensagem. Com essa CLI, você faz seu pedido diretamente no sistema do Dr. Pizza, escolhe se quer **Receber em Casa (Delivery)** ou **Buscar na Loja (Retirada)** e acompanha seus pontos de fidelidade em tempo real. Rápido, direto e sem enrolação.

```bash
curl -L -o drpizza https://github.com/raissonsouto/drpizza-cli/releases/latest/download/drpizza_linux_amd64 && chmod +x drpizza && sudo mv drpizza /usr/local/bin/
```

Agora, basta digitar `drpizza` em qualquer lugar do terminal para pedir sua pizza!

## Funcionalidades

O sistema conecta você diretamente às unidades das Malvinas, Cruzeiro e Alto Branco.

* **Pedido Real**: Monte seu carrinho, escolha bordas e envie o pedido para a cozinha.
* **Delivery ou Retirada**: Defina se o motoboy leva até você ou se você passa para pegar.
* **Cardápio Atualizado**: Acesso instantâneo aos preços, promoções e novos sabores.
* **Programa de Fidelidade**: Consulte seu saldo de pontos e troque por recompensas (Refrigerantes, Descontos) direto pelo terminal.
* **Multi-Lojas**: Selecione a unidade da Dr. Pizza mais próxima da sua casa.

## Contribua (Rodar Localmente)

Se você quer contribuir com o código ou rodar a versão de desenvolvimento na sua máquina:

### 1. Baixe o repositório

```bash
git clone https://github.com/raissonsouto/drpizza-cli.git
cd drpizza-cli
```

### 2. Compile e Rode

Se você tiver o `make` configurado (opcional) ou usando o Cargo diretamente:

```bash
# Compilar versão de produção
cargo build --release

# Rodar diretamente
cargo run
```
