import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { PROGRAM_ID } from "../utils/constants";

const idl = require("../../target/idl/token_quest.json");

export async function withdrawFeesSol() {
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);

  const [feePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("fee"), Buffer.from("sol")],
    program.programId
  );

  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("state")],
    program.programId
  );

  try {
    const tx = await program.methods
      .withdrawFeesSol()
      .accounts({
        admin: provider.wallet.publicKey,
        feePda,
        state: statePda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ withdrawFeesSol tx:", tx);
  } catch (err) {
    console.error("withdrawFeesSol failed:", err);
    throw err;
  }
}
