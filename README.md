# Raftt-Back-end

Programa Anchor da Raftt para criar ofertas SPL, receber investimentos, cancelar
ofertas abertas com reembolso individual e liberar fundos quando a meta for
atingida.

## Desenvolvimento local

O programa usa um `offering_id` (`u64`) fornecido pelo criador. O ID compõe a
PDA da oferta junto da autoridade:

`["offering", authority, offering_id.to_le_bytes()]`

Uma mesma carteira pode criar várias ofertas usando IDs diferentes. O mesmo ID
para a mesma carteira deriva o mesmo endereço, portanto não pode ser usado para
criar uma segunda conta de oferta independente.

Compile e execute os testes locais com:

```sh
anchor build
cargo test -p raftt
cargo fmt --all -- --check
git diff --check
```

O `anchor build` gera o programa e o IDL em `target/idl/raftt.json`, que deve
ser usado pelo cliente quando a integração for iniciada.

## Primeiro deploy de demonstração

O projeto ainda está configurado para `localnet` em `Anchor.toml`. Antes do
primeiro deploy:

1. Confirme a carteira de deploy e financie-a com SOL de teste na devnet.
2. Configure a rede devnet no Anchor ou passe `--provider.cluster devnet` ao
   comando de deploy.
3. Execute `anchor build` e revise o IDL gerado.
4. Faça o deploy explicitamente na devnet; não use uma carteira ou cluster de
   produção para este teste.
5. Registre o program ID implantado e valide na devnet criação de ofertas,
   investimentos, cancelamento, reembolso de cada investidor e liberação de
   uma oferta concluída.

Não foi feito deploy por este repositório nem se presume que haja contas
implantadas: os testes atuais usam LiteSVM local.

## Compatibilidade de contas

O `offering_id` faz parte da seed da oferta e também foi adicionado ao layout da
conta `Offering`. Alterações futuras em seeds ou layouts podem tornar contas
existentes inacessíveis ou incompatíveis. Antes de atualizar um programa já
implantado com dados que precisem ser preservados, planeje e teste uma
migração; um redeploy descartável só é apropriado quando as contas e os fundos
antigos não precisam ser mantidos.
