# RAFTT — rodadas v2 e parcelas

Implementação do MVP com regras próprias da RAFTT. Não implementa Reg CF.

## O que mudou

Backend: `programs/raftt/src/v2.rs`, funções adicionais no `lib.rs`, erros e `tests/test_round_v2.rs`.
Frontend: página `/round-v2`, cliente e interface `assets/js/solana/v2-*.js`, IDL ampliado e links nas páginas existentes.

A v1 foi preservada para suas ofertas já criadas. Suas instruções não conseguem acessar contas v2: tipos/discriminadores e seeds diferentes. A v1 continua tendo liberação integral pela criadora; as novas rodadas devem ser criadas em `/round-v2`. Os cards e o editor demonstrativo anterior não foram convertidos automaticamente em dados on-chain.

## Regras implementadas

1. A responsável pelo deploy inicializa `PlatformConfig` com uma carteira autorizada da RAFTT. O contrato verifica a upgrade authority do próprio programa; uma startup não pode tomar essa configuração.
2. A startup cria um rascunho com identificador, mint SPL Token, mínimo, objetivo/limite, prazo, destinatário e hash do plano. Pode criar várias rodadas com identificadores diferentes. O prazo precisa ficar a mais de 48 horas no futuro.
3. A startup cadastra parcelas sequenciais. O orçamento do plano inicial precisa somar exatamente o mínimo para a RAFTT abrir a captação. Isso garante um plano financiável quando a rodada capta apenas o mínimo. As parcelas existentes não podem ter seus valores alterados.
4. A RAFTT registra o hash da análise e aprova a abertura. A carteira da RAFTT deve ser diferente da criadora. O frontend permite registrar o relatório preparado pela IA e a decisão da RAFTT.
5. Investidores aportam até o objetivo/limite. Atingir esse valor não fecha antecipadamente a rodada.
6. Cancelamento parcial ou total só é permitido antes de `prazo - 48h`. No instante exato de início da janela, já fica bloqueado. Cancelamentos reduzem a posição e o valor captado líquido.
7. Após o prazo, qualquer pessoa pode executar o encerramento. Não existe um agendador neste pacote: a condição depende do relógio on-chain, mas uma transação ainda precisa executar `close_round`. Aportes após o prazo são bloqueados mesmo se ninguém tiver encerrado ainda.
8. Abaixo do mínimo, a rodada falha. O investidor pode reembolsar sua posição integral, uma única vez. O valor captado no encerramento fica como histórico; saldo do cofre e posição diminuem com reembolsos.
9. Com mínimo atingido, a captação conclui. Os fundos continuam no cofre.
10. A startup submete o hash de uma solicitação/evidências de uma parcela. A RAFTT registra o hash da análise e autoriza. Somente após isso o contrato permite pagar a parcela, em ordem, para uma conta SPL do destinatário fixado na rodada.
11. A execução de uma parcela já autorizada é permissionless: qualquer pessoa pode pagar a taxa da transação, mas não mudar valor, destinatário ou aprovação. Cada parcela só é paga uma vez.
12. Excedente acima do mínimo permanece no cofre. A startup pode propor parcelas adicionais até o total captado; também precisam de solicitação, análise e autorização da RAFTT. Não existe retirada automática do excedente.
13. A RAFTT pode cancelar uma rodada em rascunho ou captação, tornando o reembolso disponível. Não pode cancelar uma rodada já concluída ou recuperar dinheiro já liberado por essa instrução.

## Validação neste ambiente

PASSOU: testes JavaScript v1 e v2, serialização u64/u16, layouts, discriminadores, sintaxe e 21 rotas/arquivos estáticos. A formatação é verificada usando a `.prettierignore` do frontend.

NÃO EXECUTADO: compilação Rust/Anchor, testes LiteSVM e deploy. Este ambiente não possui Rust, Anchor ou Solana. Portanto, este código ainda precisa ser compilado e testado no seu WSL antes de considerar a v2 funcionando on-chain. Não acompanha `.so` nem chave privada.

A suíte nova verifica: autoridade e orçamento; limite que não encerra automaticamente; fronteira exata de cancelamento; prazo; rodada abaixo do mínimo; reembolso duplicado; liberação prematura, aprovação indevida, ordem, destinatário e pagamento duplicado. O bootstrap positivo deve também ser validado no validator: a fixture LiteSVM injeta apenas o cadastro da plataforma para testar o restante do fluxo.

## Instalar e testar no WSL

Copie `backend/programs` e arquivos de configuração para uma cópia do seu projeto. Preserve seu `target/deploy/raftt-keypair.json`: este pacote não inclui nem substitui essa chave.

```bash
cd ~/raftt
cargo fmt --all
cargo build-sbf --manifest-path programs/raftt/Cargo.toml --arch v0
cargo test
anchor idl build
```

O `Cargo.lock` foi copiado do projeto original. Como adicionamos dependências de teste, o Cargo pode atualizá-lo; não use `--locked` nesta primeira execução. Se `anchor idl build` não existir na versão instalada, consulte `anchor idl --help` e gere o IDL com a ferramenta instalada. Não use o IDL antigo que contém apenas o contador.

Compare o IDL gerado com a interface do frontend:

```bash
python3 verify-idl.py target/idl/raftt.json
```

O arquivo `idl-client-reference.json` foi construído a partir da interface do código e verificado em JS; não é um IDL produzido pelo compilador. O IDL gerado pela sua compilação é a referência final. O verificador aceita diferenças de nomes em camelCase/snake_case, mas rejeita diferenças de ordem, tipos, flags e discriminadores. Não faça o deploy se essa comparação falhar.

Com o validator rodando em outro terminal:

```bash
solana program deploy target/deploy/raftt.so \
  --program-id target/deploy/raftt-keypair.json \
  --keypair ~/.config/solana/id.json \
  --url http://127.0.0.1:8899
```

O ID continua `5sSTJ3RXRcz8GmmtuSwq6epjJJQZagCCi3AaEwpffmhZ`, se você preservar a keypair correspondente. Não execute `--reset` no validator que contém seus testes antigos.

## Registrar a RAFTT sem importar a carteira do deploy no Phantom

Depois do deploy, registre uma carteira de teste da RAFTT que você consiga conectar no navegador. No WSL:

```bash
cd ~/raftt/tools
npm install
npm run setup-platform -- ENDERECO_PUBLICO_DA_CARTEIRA_RAFTT
```

O utilitário usa `~/.config/solana/id.json` somente no seu computador. Não envia nem imprime a chave privada. A carteira indicada precisa ser diferente da startup. O utilitário não foi executado aqui porque precisa do programa publicado e da sua assinatura local. Se usar esse comando, não repita a configuração pela tela.

## Frontend

Na pasta `frontend`:

```bash
npm install
npm run check
npm run test:solana
npm run test:v2
npm start
```

Abra `http://localhost:8765/round-v2`. O RPC continua `http://127.0.0.1:8899` em `assets/js/solana/config.js`. O navegador precisa conseguir alcançar o validator no seu computador. Publicar o frontend na Vercel não publica o validator; para testes compartilhados é necessário deploy em devnet e configuração correspondente.

### Roteiro de teste

1. Conecte a carteira responsável pelo deploy. Em "Configuração inicial", cadastre a carteira de teste da RAFTT — guarde acesso a ela. Essa configuração é inicializada uma vez; rotação de autoridade não está implementada nesta versão.
2. Conecte a startup e crie uma rodada: mínimo 10, objetivo 20, prazo maior que 48h, destinatário startup.
3. Anote o endereço da rodada. Cadastre parcelas de 4 e 6 tokens.
4. Conecte a carteira RAFTT, consulte o endereço, forneça o relatório/decisão e abra a rodada.
5. Conecte o investidor com SOL e tokens desse mint; consulte a mesma rodada e invista.
6. Teste cancelamento antes da janela. Para testar prazos sem esperar dias, use a suíte LiteSVM. Não encurte a constante de produção apenas para testar.
7. Depois do encerramento bem-sucedido, a startup envia a solicitação da parcela; a RAFTT autoriza; execute o pagamento. Observe redução no cofre e aumento no total liberado.
8. Em outra rodada abaixo do mínimo, execute o encerramento e o reembolso.

## IA, documentos e serviços ainda pendentes

A autoridade de pagamento é a carteira da RAFTT. Esta entrega registra hashes SHA-256 de planos, solicitações/evidências e análises. Um hash sozinho não prova que uma IA foi executada nem que uma evidência é verdadeira.

Não foi disponibilizado um backend de IA/login/storage nem credenciais do provedor. Portanto, o pacote não chama um modelo e não guarda arquivos/relatórios em banco. O relatório precisa ser inserido manualmente pelo responsável da RAFTT nesta versão. Preserve os documentos exatos fora da blockchain: um hash não permite recuperar o conteúdo.

O próximo serviço deve guardar o conteúdo e os arquivos, executar a análise da IA, apresentar o relatório para a RAFTT e então solicitar a assinatura da autorização. Chaves privadas não devem ficar no frontend. Aprovação automática, sem revisão humana, exigirá um serviço assinante e critérios definidos; não foi ativada implicitamente.

Também não há emissão de equity/SAFE, KYC, taxas, mercado secundário ou retorno financeiro implementados. A posição representa tokens aportados, e não uma participação societária calculada.
