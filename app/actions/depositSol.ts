import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PROGRAM_ID } from "../utils/constants";


const idl = require("../../target/idl/token_quest.json");


export async function depositSol(amountLamports: number) {
const provider = anchor.getProvider() as anchor.AnchorProvider;
const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);


const [vaultPda] = PublicKey.findProgramAddressSync([
Buffer.from("vault"),
Buffer.from("sol"),
], program.programId);


const [stakePda] = PublicKey.findProgramAddressSync([
Buffer.from("stake"),
provider.wallet.publicKey.toBuffer(),
], program.programId);


const [statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], program.programId);


try {
const tx = await program.methods
.depositSol(new anchor.BN(amountLamports))
.accounts({
user: provider.wallet.publicKey,
vaultPda,
stakePda,
state: statePda,
systemProgram: SystemProgram.programId,
})
.rpc();


console.log("✅ depositSol tx:", tx);
console.log("stake pda:", stakePda.toBase58());
} catch (err) {
console.error("depositSol failed:", err);
throw err;
}
