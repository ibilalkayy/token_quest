import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PROGRAM_ID } from "../utils/constants";

const idl = require("../../target/idl/token_quest.json");

export async function initialize() {
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);

  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("state")],
    program.programId
  );

  try {
    const tx = await program.methods
      .initialize()
      .accounts({
        state: statePda,
        admin: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ initialize tx:", tx);
    console.log("state pda:", statePda.toBase58());
  } catch (err) {
    console.error("initialize failed:", err);
    throw err;
  }
}
