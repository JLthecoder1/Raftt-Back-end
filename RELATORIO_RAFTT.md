# Raftt — Relatório Técnico Completo

**Responsável:** Vyctor Rodrigues  
**Data de referência:** 11/10/2026  
**Tecnologias:** Rust, Anchor, Solana, SPL Token, LiteSVM e Git/GitHub  
**Estado:** Contrato implantado na Solana Devnet; integração com o front-end em desenvolvimento.

---

## Sumário

1. Objetivo do projeto
2. Ambiente e ferramentas
3. Estrutura do repositório
4. Arquitetura e modelo de dados
5. Instruções do contrato
6. Testes locais
7. Implantação na Solana Devnet
8. IDL do programa
9. Testes end-to-end na Devnet
10. Endereços e transações
11. Custos observados
12. Git e segurança
13. Integração com o front-end
14. Limitações e riscos
15. Próximos passos
16. Referências

---

## 1. Objetivo do projeto

Raftt é um projeto de contrato inteligente desenvolvido em Rust com Anchor para a blockchain Solana. O objetivo é gerenciar ofertas de investimento usando tokens SPL.

O contrato implementa os seguintes fluxos:

- Criação de ofertas com uma meta de arrecadação.
- Recebimento de investimentos em tokens SPL.
- Cancelamento de uma oferta por falha.
- Reembolso individual dos investidores.
- Fechamento da oferta ao atingir a meta.
- Liberação dos tokens de uma oferta concluída.

O projeto evoluiu de testes locais para implantação na Devnet e validação de operações on-chain.

**Importante:** a Devnet é uma rede de testes. O projeto não deve ser considerado pronto para receber ativos reais sem revisão de segurança independente.

## 2. Ambiente e ferramentas

| Componente | Informação |
|---|---|
| Sistema operacional de desenvolvimento | Ubuntu/WSL |
| Linguagem | Rust |
| Framework blockchain | Anchor |
| Rede utilizada | Solana Devnet |
| Solana CLI | 4.1.2 |
| Anchor CLI | 1.2.0 |
| SPL Token CLI | 5.6.1 |
| Node.js | 22.22.1 |
| npm | 10.9.3 |
| Testes locais | Cargo e LiteSVM |
| Diretório do projeto | `~/raftt` |
| Branch Git | `main` |
| Remoto Git | `origin/main` |

As versões acima são as registradas durante o desenvolvimento e podem ter mudado posteriormente.

### Configuração do Anchor

O arquivo `Anchor.toml` contém a configuração do programa na Devnet:

```toml
[programs.devnet]
raftt = "7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85"

[provider]
cluster = "devnet"
wallet = "~/.config/solana/id.json"
```

O caminho da carteira é uma configuração local. O arquivo `id.json` contém material privado e **nunca deve ser enviado ao GitHub**.

## 3. Estrutura do repositório

Estrutura conhecida do projeto:

```text
raftt/
├── Anchor.toml
├── Cargo.toml
├── Cargo.lock
├── README.md
├── rust-toolchain.toml
├── app/
└── programs/
    └── raftt/
        ├── src/
        │   ├── lib.rs
        │   ├── instructions/
        │   │   ├── initialize.rs
        │   │   ├── invest.rs
        │   │   ├── fail_offering.rs
        │   │   ├── refund.rs
        │   │   └── release.rs
        │   └── state/
        │       └── state.rs
        └── tests/
            ├── test_initialize.rs
            └── test_invest.rs
```

### Arquivos importantes

- `Anchor.toml`: configuração do Anchor e do cluster.
- `Cargo.toml`: dependências e configuração do projeto Rust.
- `Cargo.lock`: versões resolvidas das dependências.
- `programs/raftt/src/lib.rs`: entrada principal do programa.
- `programs/raftt/src/instructions/`: lógica das instruções do contrato.
- `programs/raftt/src/state/state.rs`: estrutura de estado das ofertas.
- `programs/raftt/tests/`: testes automatizados.
- `target/idl/raftt.json`: IDL gerada pelo Anchor.
- `target/deploy/raftt.so`: binário compilado do programa.

A pasta `target/` contém artefatos gerados e normalmente deve ser ignorada pelo Git. Arquivos de chave privada também devem permanecer fora do repositório.

## 4. Arquitetura e modelo de dados

O programa utiliza **PDA (Program Derived Address)** para associar ofertas, cofres e posições de investidores.

### 4.1. Offering

A conta `Offering` representa uma oferta de investimento. Os campos registrados na implementação são:

| Campo | Finalidade |
|---|---|
| `authority` | Autoridade associada à oferta |
| `offering_id` | Identificador da oferta |
| `mint` | Mint do token SPL utilizado |
| `target_amount` | Meta de arrecadação |
| `raised_amount` | Total arrecadado |
| `is_closed` | Indica se a oferta está fechada |
| `is_failed` | Indica se a oferta falhou ou foi cancelada |
| `is_released` | Indica se os tokens foram liberados |
| `vault` | Referência ao cofre |
| `bump` | Bump da PDA |

### 4.2. Derivação das PDAs

| Conta | Seeds descritas na implementação |
|---|---|
| Offering | `b"offering"`, chave da autoridade e `offering_id` em little-endian |
| Vault | `b"vault"` e chave da oferta |
| Position | `b"position"`, chave da oferta e chave do investidor |

A conta `Position` representa a posição de um investidor em uma oferta. No fluxo de reembolso testado, o valor da posição é zerado após a devolução dos tokens, impedindo que o mesmo saldo seja reembolsado novamente.

**Atenção à compatibilidade:** alterar seeds ou o layout das contas pode afetar contas já existentes na blockchain. Antes de atualizar o programa, é necessário avaliar a compatibilidade com os dados on-chain.

---

*Continua na Parte 2: instruções do contrato, testes locais, implantação na Devnet e publicação do IDL.*

## 5. Instruções do contrato

O contrato foi organizado em instruções separadas, cada uma responsável por uma operação do ciclo de vida de uma oferta.

### 5.1. `initialize`

Cria uma oferta de investimento com um identificador e uma meta de arrecadação.

Parâmetros conhecidos:
- `offering_id`: identificador numérico da oferta (`u64`).
- `target_amount`: meta de arrecadação (`u64`).

A instrução inicializa o estado da oferta e suas contas associadas, conforme as regras implementadas no programa.

### 5.2. `invest`

Permite que um investidor deposite tokens SPL na oferta.

O fluxo validado inclui:
- Transferência dos tokens para o cofre da oferta.
- Atualização do valor investido pelo participante.
- Atualização do total arrecadado.
- Fechamento da oferta quando a meta é atingida.

O comportamento precisa impedir investimentos em ofertas que já estejam fechadas ou canceladas.

### 5.3. `fail_offering`

Permite à autoridade cancelar uma oferta ainda aberta.

O comportamento validado na Devnet inclui:
- Cancelamento da oferta.
- Rejeição de novos investimentos.
- Rejeição de uma segunda tentativa de cancelamento.
- Bloqueio da liberação de tokens de uma oferta cancelada.

### 5.4. `refund`

Devolve ao investidor o saldo correspondente à sua própria posição em uma oferta cancelada.

O fluxo validado inclui:
- Reembolso individual.
- Separação dos saldos de cada investidor.
- Zeragem da posição após o reembolso.
- Esvaziamento do cofre após todos os reembolsos do cenário testado.

Esse comportamento é importante para impedir que um investidor receba o saldo pertencente a outro participante.

### 5.5. `release`

Libera os tokens arrecadados para o destino da autoridade após a conclusão da oferta.

No cenário testado:
- A meta foi atingida.
- O cancelamento e o reembolso foram rejeitados.
- A liberação transferiu os tokens ao destino da autoridade.
- O cofre terminou com saldo zero.

A operação deve ser permitida somente quando as condições de conclusão forem satisfeitas e não deve permitir uma segunda liberação.

### 5.6. Máquina de estados simplificada

Uma oferta pode seguir dois caminhos principais:

```text
                    initialize
                        |
                        v
                   Oferta aberta
                    /         \
                   /           \
          fail_offering       invest
                 |               |
                 v               v
          Oferta cancelada   Meta atingida
                 |               |
                 v               v
              refund           release
                 |               |
                 v               v
          Cofre esvaziado   Cofre esvaziado
```

O diagrama representa os fluxos de negócio testados, não todas as possíveis transições internas do programa.

## 6. Testes locais

Os testes locais foram utilizados para verificar a lógica do contrato sem depender de cada operação ocorrer na Devnet.

### 6.1. Ferramentas

- Cargo para execução dos testes Rust.
- Anchor para compilação e desenvolvimento do programa.
- LiteSVM para simular operações em um ambiente Solana.
- Testes de inicialização e investimento nos arquivos de teste do projeto.

### 6.2. Resultados registrados

| Verificação | Resultado informado |
|---|---|
| `anchor build` | Sucesso |
| `cargo test -p raftt` | 28 testes aprovados |
| `cargo fmt --all -- --check` | Sucesso |
| `git diff --check` | Sucesso |

Os 28 testes aprovados correspondem à execução registrada durante o desenvolvimento. A quantidade pode mudar conforme novos testes sejam adicionados.

### 6.3. Comandos úteis

Compilar o programa:

```bash
anchor build
```

Executar os testes do pacote:

```bash
cargo test -p raftt
```

Verificar a formatação:

```bash
cargo fmt --all -- --check
```

Verificar problemas de whitespace no diff:

```bash
git diff --check
```

**Diferença importante:** testes locais com LiteSVM verificam comportamentos em ambiente simulado. Eles não substituem testes de integração na Devnet nem uma auditoria de segurança.

## 7. Implantação na Solana Devnet

Após a implementação e os testes locais, o programa foi compilado e implantado na Solana Devnet.

### 7.1. Identificação do programa

| Propriedade | Valor |
|---|---|
| Programa | Raftt |
| Program ID | `7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85` |
| Rede | Solana Devnet |
| Upgradeable | Sim, conforme consulta registrada |
| Estado de verificação no Explorer | Não verificado no momento da consulta |

A indicação de programa não verificado no Explorer não significa, por si só, que o contrato esteja quebrado. Significa que o processo de verificação do build não havia sido concluído no momento observado.

### 7.2. Consulta do programa

O programa pode ser consultado no Explorer:

https://explorer.solana.com/address/7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85?cluster=devnet

Comandos úteis para consultar o programa:

```bash
solana program show 7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85 --url devnet
```

Verificar o cluster configurado:

```bash
solana config get
```

O endereço público do programa pode ser compartilhado. A chave privada da carteira de implantação não deve ser compartilhada.

## 8. Publicação do IDL

Durante a configuração da integração, a tentativa de buscar o IDL on-chain apresentou um problema porque a conta de metadados do IDL ainda não havia sido inicializada.

O IDL local estava em:

```text
target/idl/raftt.json
```

A inicialização foi executada com:

```bash
anchor idl init \
  --filepath target/idl/raftt.json \
  7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85
```

A operação foi concluída e criou a conta de metadados do IDL.

Endereço registrado para essa conta:

```text
FbfYY57Naqp1FJiZsbH7STp2nMWCYwownpx2xzeGcjBW
```

Depois disso, a consulta do IDL passou a funcionar. A comparação entre o arquivo local e o IDL recuperado mostrou somente uma diferença de quebra de linha no final do arquivo.

Esse processo é relevante porque o front-end precisa conhecer as instruções, contas e tipos definidos pelo programa para interagir com o contrato.

*Fim da Parte 2.*

---

## 9. Testes end-to-end na Devnet

Em 11/10/2026, foram executados testes end-to-end na Solana Devnet, utilizando um mint SPL de teste criado para os cenários. O objetivo foi validar o comportamento do contrato na rede, incluindo as transferências de tokens e as restrições de estado.

Foi informado que o bytecode implantado correspondia ao build local utilizado antes dos testes.

Os testes utilizaram um mint fictício de Devnet, sem tocar no depósito anterior mencionado durante o desenvolvimento.

### 9.1. Cenário A — Oferta cancelada e reembolsos

O cenário validou o comportamento de uma oferta cancelada após receber investimentos de dois participantes.

Sequência executada:

1. Criação da oferta.
2. Depósito de tokens por dois investidores.
3. Cancelamento da oferta pela autoridade por meio de `fail_offering`.
4. Tentativa de novo investimento, rejeitada.
5. Tentativa de cancelar novamente, rejeitada.
6. Reembolso do primeiro investidor.
7. Reembolso do segundo investidor.
8. Verificação do saldo final do cofre.

Resultado relatado:

- A oferta foi cancelada.
- Novos investimentos foram bloqueados.
- O segundo cancelamento foi rejeitado.
- Cada investidor recebeu apenas o valor correspondente à sua própria posição.
- O cofre terminou com saldo zero.
- A liberação dos tokens de uma oferta cancelada foi rejeitada.

Esse cenário cobre uma parte fundamental da lógica de segurança: o contrato não deve permitir novos depósitos após o cancelamento e não deve misturar os saldos individuais dos participantes.

### 9.2. Cenário B — Oferta concluída e liberação

O segundo cenário validou o fluxo de conclusão da oferta.

Sequência executada:

1. Criação de uma nova oferta.
2. Depósitos de tokens até atingir a meta.
3. Verificação do fechamento da oferta.
4. Tentativas de cancelamento e reembolso, ambas rejeitadas.
5. Execução de `release`.
6. Verificação do saldo final do cofre.

Resultado relatado:

- A meta foi atingida.
- A oferta foi fechada.
- O cancelamento foi rejeitado após a conclusão.
- O reembolso foi rejeitado no cenário de oferta concluída.
- A liberação transferiu os tokens para o destino da autoridade.
- O cofre terminou com saldo zero.

### 9.3. Interpretação dos resultados

Os dois cenários exercitaram transições distintas do ciclo de vida de uma oferta:

| Comportamento | Cancelamento | Conclusão |
|---|---|---|
| Receber investimentos antes do encerramento | Sim | Sim |
| Bloquear novos investimentos depois do encerramento | Validado | Validado |
| Permitir cancelamento | Sim, antes do encerramento | Não após a conclusão |
| Permitir reembolso | Sim, conforme a posição de cada investidor | Não no cenário concluído |
| Permitir liberação | Não | Sim |
| Saldo final do cofre | Zero | Zero |

Os resultados acima são baseados nas execuções relatadas durante o desenvolvimento. Uma revisão independente do código e das transações continua sendo necessária antes de considerar o sistema seguro para uso com ativos reais.

## 10. Endereços públicos e transações

Os links abaixo permitem consultar os registros públicos na Solana Explorer, selecionando a rede Devnet.

### 10.1. Identificadores utilizados

| Elemento | Endereço |
|---|---|
| Programa Raftt | `7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85` |
| Mint SPL de teste | `APxmiNX8PRTsLNZeKgkRdRfDVtwaWoFeHk1tJkwWXvZC` |
| Oferta cancelada | `H4tFD9WG4Eh1NHiu8hYvHuGHKTng6vEvJqcGJwptc8Sn` |
| Oferta concluída e liberada | `FvJWNyP2LW4xuqc1Zs7TUt49V78GmoRmVp9f21mwKsaH` |
| Carteira temporária de investidor | `FHyFmv7bCpWt6vDrsEr9nrcQdbXnj5pjV84cYd2H1SV6` |
| Conta de metadados do IDL | `FbfYY57Naqp1FJiZsbH7STp2nMWCYwownpx2xzeGcjBW` |

Esses endereços são identificadores públicos de contas e não substituem a inspeção individual das transações.

### 10.2. Transações principais

**Criação da oferta cancelada**

https://explorer.solana.com/tx/2ECr2LK144soRdzt7Lwq37BosY7HVtaSvE6SkgneKLY2TEKUxzWDZHQjLtQYvdTjgqaozxQRzXzt6mAf9mW6s8fm?cluster=devnet

**Investimento do segundo participante**

https://explorer.solana.com/tx/2vrT9zXb72wJRH2ANc67srW9syswmwNaCQ7Q92XnXAPppkNz15AEQd6LwJx9tPw2h7WGrazSzKexPJ5UsbbuPGeU?cluster=devnet

**Cancelamento da oferta**

https://explorer.solana.com/tx/3kUFRbqpCHwZMaV26LGpZanMp33YZMehCAtif8kMwWZDEq6UtHikutDAWmAoPLTcZ48uEkEckpF52VxEYRgt8Zwg?cluster=devnet

**Reembolso do primeiro investidor**

https://explorer.solana.com/tx/5z5WhgoxnEovaAWqRjg4BwNUPR1tZkM5emtEAJHrbSXfCBgm8j9EmbDV7peJeynoUp4CkGyydaKBgMCo88xumy7G?cluster=devnet

**Reembolso do segundo investidor**

https://explorer.solana.com/tx/59Dc3DeyGjaNkmHqmQH8An14qSBfNY198DMqJ11C3ysAC86Yj8N1JNn5zQVha5Ws8T9qhH2EF1oNEq6zFyMPcPML?cluster=devnet

**Liberação da oferta concluída**

https://explorer.solana.com/tx/4aCxH6bC5icYxUFvkCcJYmwtRGUENTMQ5w7XxGqKR3aFAu9Z92rBBPrmbwkpQ3bFL44fryo1vzN7KTbGdWoqj2dd?cluster=devnet

### 10.3. Como conferir uma transação

Ao abrir cada link no Explorer, confira:

- Status da transação.
- Instruções executadas.
- Programa responsável pela execução.
- Contas envolvidas.
- Transferências de tokens.
- Horário e assinatura da transação.

Uma transação bem-sucedida demonstra que aquela execução foi aceita pela rede. Isso, isoladamente, não prova a ausência de vulnerabilidades em todas as outras condições possíveis.

*Fim da Parte 3.*

---

## 11. Custos observados durante os testes

Durante os testes na Devnet, foi registrado o seguinte saldo da carteira principal:

| Medição | Saldo |
|---|---:|
| Saldo inicial informado | 1,297527707 SOL |
| Saldo final informado | 1,275652107 SOL |
| Variação | -0,021875600 SOL |

A diferença corresponde à redução total observada entre as duas medições. Ela não representa apenas taxas de transação.

O valor inclui taxas de rede, custos de criação de contas que exigem saldo mínimo e uma transferência de `0,01 SOL` para uma carteira temporária de investidor. Não foi feito um detalhamento contábil completo de cada parcela.

Os SOL utilizados na Devnet são tokens de teste, sem equivalência direta com SOL da rede principal.

## 12. Git, GitHub e colaboração

O projeto foi desenvolvido em um repositório Git, com a branch principal chamada `main` e o remoto `origin/main`.

O histórico incluiu alterações em:

- Instruções do contrato.
- Estado das ofertas.
- Testes automatizados.
- Configuração do projeto.
- `.gitignore`.
- `README.md`.

Um dos commits registrados durante o desenvolvimento foi:

```text
c9bc6e7 test: reforça validações de falha e reembolso
```

Esse commit fazia parte do histórico observado durante a etapa de desenvolvimento. O repositório posteriormente recebeu atualizações.

### 12.1. Comandos utilizados no fluxo de colaboração

Verificar o estado do repositório:

```bash
git status
```

Revisar as alterações:

```bash
git diff
```

Preparar arquivos para o commit:

```bash
git add .gitignore Anchor.toml README.md programs/raftt/src programs/raftt/tests
```

Conferir o conteúdo preparado:

```bash
git diff --cached --check
git diff --cached
```

Criar o commit:

```bash
git commit -m "feat: implementa fluxo de investimento e reembolso"
```

Enviar as alterações:

```bash
git push origin main
```

Esses comandos documentam o fluxo utilizado. Não devem ser executados novamente apenas por constarem neste relatório: antes de qualquer novo commit, confira o estado atual do repositório.

### 12.2. Arquivos ignorados

O `.gitignore` foi organizado para evitar o envio de artefatos de compilação, dependências e arquivos sensíveis.

Exemplo de configuração:

```gitignore
# Anchor / Solana
/.anchor/
/target/
test-ledger/

# Node.js
node_modules/

# Rust
**/*.rs.bk

# Segredos e credenciais
.env
.env.*
!.env.example
*.pem
*.key
*-keypair.json
id.json

# Sistema e temporários
.DS_Store
*.log
.yarn/
.surfpool/
```

**Importante:** adicionar um arquivo ao `.gitignore` não remove automaticamente arquivos que já estejam rastreados pelo Git. Também não apaga segredos que já tenham sido publicados no histórico. Se uma chave privada tiver sido exposta, é necessário tratá-la como comprometida e tomar medidas de segurança.

## 13. Integração com o front-end

No momento da documentação, outro integrante da equipe estava desenvolvendo o front-end.

O contrato já possuía um Program ID na Devnet e um IDL publicado, elementos importantes para permitir que a aplicação cliente conheça as instruções e as contas esperadas pelo programa.

### 13.1. Informações úteis para o front-end

| Informação | Valor |
|---|---|
| Cluster | `devnet` |
| Program ID | `7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85` |
| IDL local | `target/idl/raftt.json` |
| Explorer | https://explorer.solana.com/address/7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85?cluster=devnet |

O front-end deve utilizar o IDL correspondente à versão implantada e construir corretamente as contas necessárias para cada instrução.

### 13.2. Funcionalidades que a interface pode oferecer

- Consultar e exibir ofertas.
- Mostrar meta e valor arrecadado.
- Conectar uma carteira compatível com Solana.
- Permitir a criação de ofertas, conforme as permissões do contrato.
- Permitir investimentos com tokens SPL compatíveis.
- Exibir o estado de cada oferta.
- Disponibilizar cancelamento para a autoridade quando permitido.
- Permitir que cada investidor solicite seu reembolso em uma oferta cancelada.
- Disponibilizar a liberação dos fundos à autoridade após a conclusão.
- Mostrar links das transações no Explorer.

Esses itens representam funcionalidades planejadas para a interface; a lista não significa que todas já estejam implementadas.

### 13.3. Pontos de integração a validar

1. Confirmar que o IDL do front-end corresponde ao programa implantado.
2. Confirmar a derivação das PDAs de oferta, cofre e posição.
3. Conferir os mints e as contas de token utilizados nas transferências.
4. Tratar erros de transação sem informar sucesso prematuramente.
5. Atualizar os dados da interface depois de uma transação confirmada.
6. Exibir claramente os estados de oferta aberta, cancelada, concluída e liberada.
7. Validar as permissões da autoridade e as posições dos investidores.
8. Testar o fluxo completo entre interface e contrato na Devnet.

## 14. Limitações e riscos conhecidos

O projeto passou por testes locais e por cenários end-to-end na Devnet, mas ainda existem pontos que precisam ser avaliados antes de qualquer uso com ativos reais.

### 14.1. Segurança do contrato

- **Auditoria independente:** não há registro de auditoria independente concluída.
- **Mint aceito:** é necessário confirmar se qualquer mint SPL pode ser utilizado ou se existe uma validação restritiva. Se não houver restrição, o contrato poderá aceitar tokens que não correspondam ao ativo esperado pela aplicação.
- **Aritmética:** conferir operações de soma, subtração e conversão de valores para evitar overflow, underflow e erros de unidade.
- **Autoridade:** validar cuidadosamente quem pode criar, cancelar e liberar uma oferta.
- **Contas e PDAs:** verificar proprietários, seeds, bumps e relações entre as contas fornecidas.
- **Transições de estado:** testar todas as combinações relevantes de oferta aberta, cancelada, concluída e liberada.
- **Transferências SPL:** verificar o tratamento de erros, o mint correto e as contas de origem e destino.
- **Compatibilidade:** alterações no layout de contas e nas seeds precisam considerar os dados existentes na rede.

### 14.2. Regras de negócio ainda a avaliar

- Definir se uma oferta deve ter prazo de encerramento.
- Definir o comportamento esperado quando a meta não é atingida.
- Estabelecer limites mínimos ou máximos de investimento, caso sejam necessários.
- Definir se haverá uma lista de mints autorizados.
- Documentar claramente as responsabilidades da autoridade.
- Considerar testes adicionais com múltiplos investidores e ofertas simultâneas.

A ausência de prazo foi registrada como característica da implementação descrita até aqui; não deve ser interpretada como requisito definitivo do produto.

### 14.3. Segurança operacional

- Nunca publicar carteiras privadas ou arquivos de keypair.
- Manter as chaves de desenvolvimento separadas das chaves de produção.
- Evitar compartilhar arquivos `.env` com credenciais.
- Revisar o diff antes de cada commit e push.
- Confirmar o cluster antes de enviar transações.
- Não confiar apenas no estado exibido pela interface: validar as transações e os dados on-chain.

## 15. Próximos passos

Sugestão de sequência para continuar o desenvolvimento:

- [ ] Concluir a integração do front-end com o IDL.
- [ ] Testar todas as operações pela interface.
- [ ] Conferir as validações de autoridade, mint, PDAs e contas de token.
- [ ] Adicionar testes para cenários inválidos e casos-limite.
- [ ] Documentar os requisitos e as regras de negócio definitivas.
- [ ] Verificar o processo de atualização do programa e a gestão da autoridade de upgrade.
- [ ] Revisar o README com instruções reproduzíveis de instalação, testes e implantação.
- [ ] Considerar verificação do programa no Explorer.
- [ ] Realizar revisão de segurança independente antes de qualquer implantação com ativos reais.

## 16. Referências

**Repositório do projeto**

https://github.com/JLthecoder1/Raftt-Back-end

**Solana Explorer — Programa Raftt na Devnet**

https://explorer.solana.com/address/7rxcmdNRE9Ve7ocjAzG1S5LxyPxuSQ5fifhydpRJqt85?cluster=devnet

**Documentação oficial do Anchor**

https://www.anchor-lang.com/

**Documentação oficial da Solana**

https://solana.com/docs

**Documentação do SPL Token**

https://spl.solana.com/token

---

## Conclusão

O Raftt evoluiu de uma implementação de contrato inteligente para uma versão implantada na Solana Devnet, com IDL publicado, testes automatizados locais e validação de dois fluxos principais de negócio: cancelamento com reembolso individual e conclusão com liberação dos tokens.

O projeto também avançou para a etapa de integração com um front-end desenvolvido em colaboração com outro integrante da equipe.

Os resultados demonstram progresso na implementação e nos testes do fluxo de investimento. O próximo objetivo é consolidar a integração entre cliente e contrato, ampliar a cobertura de testes e revisar a segurança antes de considerar qualquer uso em produção.

**Status final deste relatório:** documentação consolidada com base nas informações e nos resultados relatados durante o desenvolvimento. Não constitui auditoria de código nem verificação independente de todas as transações.
