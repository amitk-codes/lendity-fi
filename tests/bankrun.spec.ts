import * as anchor from "@coral-xyz/anchor";
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";
import { BankrunProvider } from "anchor-bankrun";
import { ProgramTestContext, startAnchor } from "solana-bankrun";
import { BankrunContextWrapper } from "../bankrun-utils/bankrunConnection";
import { LendityFi } from "../target/types/lendity_fi";
import LendityFiIdl from "../target/idl/lendity_fi.json"

describe("Lendity-Fi", () => {
  const web3 = anchor.web3;
  const pyth = new web3.PublicKey(PYTH_PUBLIC_ADDRESS);
  const devnetConnection = new web3.Connection(DEVNET_RPC_ENDPOINT);

  let context: ProgramTestContext;
  let provider: BankrunProvider;
  let program: anchor.Program<LendityFi>;

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
  });
});
