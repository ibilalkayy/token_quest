import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PROGRAM_ID } from "../utils/constants";

const idl = require("../../target/idl/token_quest.json");

export async function withdrawSpl(
  mint: PublicKey,
  userTokenAccount: PublicKey
) {
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);

  const [stakePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("stake"),
      provider.wallet.publicKey.toBuffer(),
      mint.toBuffer(),
    ],
    program.programId
  );

  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), mint.toBuffer()],
    program.programId
  );

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
      .withdrawSpl()
      .accounts({
        user: provider.wallet.publicKey,
        stakePda,
        mint,
        vaultPda,
        userTokenAccount,
        feePda,
        state: statePda,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("✅ withdrawSpl tx:", tx);
  } catch (err) {
    console.error("withdrawSpl failed:", err);
    throw err;
  }
}
