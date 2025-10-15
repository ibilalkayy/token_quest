import { PublicKey } from "@solana/web3.js";
import { PROGRAM_ID } from "./constants";

export async function getVaultPdaSOL(): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), Buffer.from("sol")],
    new PublicKey(PROGRAM_ID)
  );
}

export async function getStakePda(
  user: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("stake"), user.toBuffer()],
    new PublicKey(PROGRAM_ID)
  );
}

export async function getFeePdaSOL(): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fee"), Buffer.from("sol")],
    new PublicKey(PROGRAM_ID)
  );
}
