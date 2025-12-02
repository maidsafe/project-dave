import { WalletClient, PublicClient, Address, Hex } from "viem";
import { arbitrum } from "viem/chains";

const ERC20_PERMIT_ABI = [
  {
    inputs: [{ name: "account", type: "address" }],
    name: "balanceOf",
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [],
    name: "name",
    outputs: [{ name: "", type: "string" }],
    stateMutability: "view",
    type: "function",
  },
  {
    inputs: [{ name: "owner", type: "address" }],
    name: "nonces",
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
    type: "function",
  },
] as const;

export interface PermitSignature {
  v: number;
  r: Hex;
  s: Hex;
  deadline: bigint;
}

export async function signERC20Permit(
  walletClient: WalletClient,
  publicClient: PublicClient,
  tokenAddress: Address,
  spenderAddress: Address,
  value: bigint,
  deadline?: bigint
): Promise<PermitSignature> {
  const account = walletClient.account;
  if (!account) {
    throw new Error("Wallet client has no account");
  }

  const userAddress = account.address;

  // Get token details for permit
  const [tokenName, nonce] = await Promise.all([
    publicClient.readContract({
      address: tokenAddress,
      abi: ERC20_PERMIT_ABI,
      functionName: "name",
    }),
    publicClient.readContract({
      address: tokenAddress,
      abi: ERC20_PERMIT_ABI,
      functionName: "nonces",
      args: [userAddress],
    }),
  ]);

  // Set deadline (default to 1 hour from now)
  const permitDeadline = deadline || BigInt(Math.floor(Date.now() / 1000) + 3600);

  // EIP-712 domain
  const domain = {
    name: tokenName as string,
    version: "1",
    chainId: arbitrum.id,
    verifyingContract: tokenAddress,
  };

  // EIP-712 types for permit
  const types = {
    Permit: [
      { name: "owner", type: "address" },
      { name: "spender", type: "address" },
      { name: "value", type: "uint256" },
      { name: "nonce", type: "uint256" },
      { name: "deadline", type: "uint256" },
    ],
  };

  // Permit message
  const message = {
    owner: userAddress,
    spender: spenderAddress,
    value: value,
    nonce: nonce as bigint,
    deadline: permitDeadline,
  };

  // Sign the permit
  const signature = await walletClient.signTypedData({
    domain,
    types,
    primaryType: "Permit",
    message,
  });

  // Parse signature into v, r, s components
  const r = signature.slice(0, 66) as Hex;
  const s = `0x${signature.slice(66, 130)}` as Hex;
  const v = parseInt(signature.slice(130, 132), 16);

  return {
    v,
    r,
    s,
    deadline: permitDeadline,
  };
}
