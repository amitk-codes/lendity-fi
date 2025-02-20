import * as anchor from "@coral-xyz/anchor";
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";
import { BankrunProvider } from "anchor-bankrun";
import { BanksClient, ProgramTestContext, startAnchor } from "solana-bankrun";
import { BankrunContextWrapper } from "../bankrun-utils/bankrunConnection";
import { LendityFi } from "../target/types/lendity_fi";
import LendityFiIdl from "../target/idl/lendity_fi.json"
import { DEVNET_RPC_ENDPOINT, PYTH_PUBLIC_ADDRESS, SOL_USD_PRICE_FEED_ID_HEX } from "../bankrun-utils/constants"
import { createMint, mintTo } from "spl-token-bankrun"
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("Lendity-Fi", () => {
  const web3 = anchor.web3;
  const pyth = new web3.PublicKey(PYTH_PUBLIC_ADDRESS);
  const devnetConnection = new web3.Connection(DEVNET_RPC_ENDPOINT);

  let context: ProgramTestContext;
  let provider: BankrunProvider;
  let program: anchor.Program<LendityFi>;
  let banksClient: BanksClient;
  let signer: anchor.web3.Keypair
  let usdcMint: anchor.web3.PublicKey;
  let solMint: anchor.web3.PublicKey;
  let usdcTokenAccount: anchor.web3.PublicKey;
  let solTokenAccount: anchor.web3.PublicKey;

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

    banksClient = context.banksClient;
    signer = provider.wallet.payer;

    // @ts-ignore
    usdcMint = await createMint(banksClient, signer, signer.publicKey, null, 2);

    // @ts-ignore
    solMint = await createMint(banksClient, signer, signer.publicKey, null, 2);


    [usdcTokenAccount] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bank_token_account"), usdcMint.toBuffer()],
      program.programId
    );

    [solTokenAccount] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bank_token_account"), solMint.toBuffer()],
      program.programId
    );

  });

  test("Initializes the user account", async() => {
    const initUserTx = await program.methods
      .initializeUser(usdcMint)
      .accounts({signer: signer.publicKey})
      .rpc();

    console.log({initUserTx});
    
  })

  test("Initializes the USDC bank and funds it's token account", async () => {
    const initUsdcBankTx = await program.methods
      .initializeBank(new anchor.BN(1), new anchor.BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: usdcMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log({ initUsdcBankTx });

    const fundingUsdcTokenAccountTx = await mintTo(
      // @ts-ignore
      banksClient,
      signer,
      usdcMint,
      usdcTokenAccount,
      signer,
      10_000 * web3.LAMPORTS_PER_SOL
    );

    console.log({ fundingUsdcTokenAccountTx });
  });

  test("Initializes the SOL bank and funds it's token account", async () => {
    const initSolBankTx = await program.methods
      .initializeBank(new anchor.BN(1), new anchor.BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: solMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log({ initSolBankTx });

    const fundingSolTokenAccountTx = await mintTo(
      // @ts-ignore
      banksClient,
      signer,
      solMint,
      solTokenAccount,
      signer,
      10_000 * web3.LAMPORTS_PER_SOL
    );

    console.log({ fundingSolTokenAccountTx });
  });
});
