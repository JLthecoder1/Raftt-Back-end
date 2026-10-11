import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { Connection, PublicKey, Keypair, SystemProgram, Transaction, TransactionInstruction, sendAndConfirmTransaction } from '@solana/web3.js';
const reviewerText=process.argv[2];
if(!reviewerText)throw Error('Uso: npm run setup-platform -- ENDERECO_PUBLICO_DA_RAFTT');
const endpoint=process.env.RAFTT_RPC||'http://127.0.0.1:8899';
const url=new URL(endpoint);
if(!['127.0.0.1','localhost','[::1]','api.devnet.solana.com'].includes(url.hostname))throw Error('RPC deve ser localnet ou endpoint oficial devnet neste utilitário.');
const c=new Connection(endpoint,'confirmed');
const genesis=await c.getGenesisHash();
if(genesis==='5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp')throw Error('Mainnet não permitida');
const secretPath=process.env.RAFTT_DEPLOY_WALLET||resolve(homedir(),'.config/solana/id.json');
const authority=Keypair.fromSecretKey(Uint8Array.from(JSON.parse(await readFile(secretPath,'utf8'))));
const reviewer=new PublicKey(reviewerText);
const programId=new PublicKey(process.env.RAFTT_PROGRAM_ID||'5sSTJ3RXRcz8GmmtuSwq6epjJJQZagCCi3AaEwpffmhZ');
const loader=new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
const [programData]=PublicKey.findProgramAddressSync([programId.toBytes()],loader);
const [config]=PublicKey.findProgramAddressSync([Buffer.from('platform-v2')],programId);
if(await c.getAccountInfo(config))throw Error('Configuração já inicializada; este comando não a substitui.');
const discriminator=createHash('sha256').update('global:setup_platform').digest().subarray(0,8);
const ix=new TransactionInstruction({programId,keys:[
 {pubkey:authority.publicKey,isSigner:true,isWritable:true},
 {pubkey:config,isSigner:false,isWritable:true},
 {pubkey:programId,isSigner:false,isWritable:false},
 {pubkey:programData,isSigner:false,isWritable:false},
 {pubkey:SystemProgram.programId,isSigner:false,isWritable:false}
],data:Buffer.concat([discriminator,reviewer.toBuffer()])});
const signature=await sendAndConfirmTransaction(c,new Transaction().add(ix),[authority],{commitment:'confirmed',skipPreflight:false});
console.log('Autoridade RAFTT:',reviewer.toBase58());
console.log('Configuração:',config.toBase58());
console.log('Transação confirmada:',signature);
