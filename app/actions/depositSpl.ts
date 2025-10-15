import * as anchor from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PROGRAM_ID } from "../utils/constants";

const idl = require("../../target/idl/token_quest.json");

export async function depositSpl(
  mint: PublicKey,
  userTokenAccount: PublicKey,
  amount: number
) {
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = new anchor.Program(idl, new PublicKey(PROGRAM_ID), provider);

  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), mint.toBuffer()],
    program.programId
  );

  const [stakePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("stake"),
      provider.wallet.publicKey.toBuffer(),
      mint.toBuffer(),
    ],
    program.programId
  );

  const [statePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("state")],
    program.programId
  );

  try {
    const tx = await program.methods
      .depositSpl(new anchor.BN(amount))
      .accounts({
        user: provider.wallet.publicKey,
        userTokenAccount,
        mint,
        vaultPda,
        stakePda,
        state: statePda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    console.log("✅ depositSpl tx:", tx);
    console.log("stake pda:", stakePda.toBase58());
  } catch (err) {
    console.error("depositSpl failed:", err);
    throw err;
  }
}
