import * as anchor from "@coral-xyz/anchor";
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";
import { BankrunProvider } from "anchor-bankrun";
import { ProgramTestContext, startAnchor } from "solana-bankrun";
import { BankrunContextWrapper } from "../bankrun-utils/bankrunConnection";
import { LendityFi } from "../target/types/lendity_fi";
import LendityFiIdl from "../target/idl/lendity_fi.json"
import { DEVNET_RPC_ENDPOINT, PYTH_PUBLIC_ADDRESS, SOL_USD_PRICE_FEED_ID_HEX } from "../bankrun-utils/constants"
import { createMint } from "spl-token-bankrun"

describe("Lendity-Fi", () => {
  const web3 = anchor.web3;
  const pyth = new web3.PublicKey(PYTH_PUBLIC_ADDRESS);
  const devnetConnection = new web3.Connection(DEVNET_RPC_ENDPOINT);

  let context: ProgramTestContext;
  let provider: BankrunProvider;
  let program: anchor.Program<LendityFi>;
  let signer: anchor.web3.Keypair
  let usdcMint: anchor.web3.PublicKey;

  beforeAll(async () => {
    const pythAccountInfo = await devnetConnection.getAccountInfo(pyth);
    context = await startAnchor(
      "",
      [],
      [
        {
          address: pyth,
          info: pythAccountInfo,
        },
      ]
    );

    provider = new BankrunProvider(context);

    const bankrunConnection = new BankrunContextWrapper(context);

    const pythSolanaReceiver = new PythSolanaReceiver({
      wallet: provider.wallet,
      connection: bankrunConnection.connection.toConnection()
    })

    const solUsdFeedAccountAddress = pythSolanaReceiver.getPriceFeedAccountAddress(0, SOL_USD_PRICE_FEED_ID_HEX).toBase58();

    const solUsdFeedAccountPublicKey = new web3.PublicKey(solUsdFeedAccountAddress);

    const solUsdFeedAccountInfo = await devnetConnection.getAccountInfo(solUsdFeedAccountPublicKey);

    context.setAccount(solUsdFeedAccountPublicKey, solUsdFeedAccountInfo);

    program = new anchor.Program<LendityFi>(LendityFiIdl as LendityFi, provider);

    const banksClient = context.banksClient;
    signer = provider.wallet.payer;

    // @ts-ignore
    usdcMint = await createMint(banksClient, signer, signer.publicKey, null, 2);

  });

  test("Initializes the user account", async() => {
    const initUserTx = await program.methods
      .initializeUser(usdcMint)
      .accounts({signer: signer.publicKey})
      .rpc();

    console.log({initUserTx});
    
  })
});
