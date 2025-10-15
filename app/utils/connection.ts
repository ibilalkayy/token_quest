import * as anchor from "@coral-xyz/anchor";
import { Connection, clusterApiUrl } from "@solana/web3.js";

export const connection = new Connection(clusterApiUrl("devnet"), "confirmed");
export const wallet = anchor.Wallet.local();
export const provider = new anchor.AnchorProvider(connection, wallet, {
  commitment: "confirmed",
});

anchor.setProvider(provider);
