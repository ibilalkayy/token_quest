import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PROGRAM_ID } from "../utils/constants";


const idl = require("../../target/idl/token_quest.json");


export async function withdrawSol() {
const provider = anchor.getProvider() as anchor.AnchorProvider;
const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);


const [stakePda] = PublicKey.findProgramAddressSync([
Buffer.from("stake"),
provider.wallet.publicKey.toBuffer(),
], program.programId);


const [vaultPda] = PublicKey.findProgramAddressSync([
Buffer.from("vault"),
Buffer.from("sol"),
], program.programId);


const [feePda] = PublicKey.findProgramAddressSync([
Buffer.from("fee"),
Buffer.from("sol"),
], program.programId);


const [statePda] = PublicKey.findProgramAddressSync([Buffer.from("state")], program.programId);


try {
const tx = await program.methods
.withdrawSol()
.accounts({
user: provider.wallet.publicKey,
stakePda,
vaultPda,
feePda,
state: statePda,
clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
systemProgram: SystemProgram.programId,
})
.rpc();


console.log("✅ withdrawSol tx:", tx);
} catch (err) {
console.error("withdrawSol failed:", err);
throw err;
}
