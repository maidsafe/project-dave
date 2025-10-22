import {createPimlicoClient} from "permissionless/clients/pimlico";
import {http, createPublicClient, type WalletClient, type Chain, type Hex} from "viem";
import {entryPoint07Address, entryPoint08Address} from "viem/account-abstraction";
import {toSafeSmartAccount, type ToSafeSmartAccountReturnType} from "permissionless/accounts";
import {createSmartAccountClient} from "permissionless";
import {arbitrum} from "viem/chains";

// Re-export the type for use in other modules
export type {ToSafeSmartAccountReturnType};

export async function getSmartAccount(
    walletClient: WalletClient,
    chain: Chain = arbitrum
): Promise<ToSafeSmartAccountReturnType<"0.7">> {
    const publicClient = createPublicClient({
        chain,
        transport: http(),
    });

    // Get the wallet account (EOA)
    const account = walletClient.account;
    if (!account) {
        throw new Error("Wallet client has no account");
    }

    const smartAccount = await toSafeSmartAccount({
        client: publicClient,
        owners: [walletClient], // Use wallet client as signer
        entryPoint: {
            address: entryPoint07Address,
            version: "0.7",
        },
        version: "1.4.1",
    });

    console.log("Smart account address:", smartAccount.address);

    return smartAccount;
}

export async function createPimlicoSmartAccountClient(
    account: ToSafeSmartAccountReturnType<"0.7">,
    apiKey: string,
    paymasterAddress: Hex,
    chain: Chain = arbitrum
) {
    const pimlicoUrl = `https://api.pimlico.io/v2/${chain.id}/rpc?apikey=${apiKey}`;

    const pimlicoClient = createPimlicoClient({
        transport: http(pimlicoUrl),
        entryPoint: {
            address: entryPoint08Address,
            version: "0.8",
        },
    });

    const smartAccountClient = createSmartAccountClient({
        account,
        chain,
        bundlerTransport: http(pimlicoUrl),
        paymaster: {
            async getPaymasterData(parameters) {
                const gasEstimates = await pimlicoClient.estimateUserOperationGas({
                    ...parameters,
                    paymaster: paymasterAddress,
                });

                const feesPerGas = (await pimlicoClient.getUserOperationGasPrice()).fast;

                return {
                    paymaster: paymasterAddress,
                    paymasterData: "0x" as Hex,
                    paymasterPostOpGasLimit: gasEstimates.paymasterPostOpGasLimit ?? 0n,
                    paymasterVerificationGasLimit: gasEstimates.paymasterVerificationGasLimit ?? 0n,
                    maxFeePerGas: feesPerGas.maxFeePerGas ?? 0n,
                    maxPriorityFeePerGas: feesPerGas.maxPriorityFeePerGas ?? 0n,
                };
            },
            async getPaymasterStubData(parameters) {
                return {
                    paymaster: paymasterAddress,
                    paymasterData: "0x" as Hex,
                    paymasterVerificationGasLimit: 50_000n,
                    paymasterPostOpGasLimit: 20_000n,
                    maxFeePerGas: 0n,
                    maxPriorityFeePerGas: 0n,
                };
            },
        },
        userOperation: {
            estimateFeesPerGas: async () => {
                return (await pimlicoClient.getUserOperationGasPrice()).fast;
            },
        },
    });

    return smartAccountClient;
}
