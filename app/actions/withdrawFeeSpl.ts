import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PROGRAM_ID } from "../utils/constants";

const idl = require("../../target/idl/token_quest.json");

export async function withdrawFeesSpl(
  mint: PublicKey,
  adminTokenAccount: PublicKey
) {
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);

  const [feePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("fee"), mint.toBuffer()],
    program.programId
  );

  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("state")],
    program.programId
  );

  try {
    const tx = await program.methods
      .withdrawFeesSpl()
      .accounts({
        admin: provider.wallet.publicKey,
        mint,
        feePda,
        adminTokenAccount,
        state: statePda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("✅ withdrawFeesSpl tx:", tx);
  } catch (err) {
    console.error("withdrawFeesSpl failed:", err);
    throw err;
  }
}
