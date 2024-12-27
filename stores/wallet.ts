import {useAppKit, useAppKitAccount, useDisconnect} from "@reown/appkit/vue";
import {readContract, waitForTransactionReceipt, writeContract, signMessage} from "@wagmi/core";
import tokenAbi from "~/assets/abi/PaymentToken.json";
import paymentVaultAbi from "~/assets/abi/IPaymentVault.json";
import {wagmiAdapter} from "~/config";

const tokenContractAddress = "0xBE1802c27C324a28aeBcd7eeC7D734246C807194";
const paymentVaultContractAddress = "0x993C7739f50899A997fEF20860554b8a28113634";
const VAULT_SECRET_KEY_SEED = "Massive Array of Internet Disks Secure Access For Everyone";

export const useWalletStore = defineStore("wallet", () => {
    // State
    const pendingConnectWallet = ref(false);
    const pendingDisconnectWallet = ref(false);
    const openConnectWallet = ref(false);
    const openDisconnectWallet = ref(false);
    const callbackConnectWallet = ref<Function | null>(null);
    const callbackDisconnectWallet = ref<Function | null>(null);

    const wallet = useAppKitAccount();
    const {open} = useAppKit();
    const {disconnect} = useDisconnect();

    const connectWallet = async () => {
        try {
            pendingConnectWallet.value = true;

            const connectResponse = await open();

            console.log(">>> Connect response: ", connectResponse);

            // TODO: Fix how to handle a response using appKit
            return {
                success: true,
            };
        } catch (error) {
            return {
                success: false,
                message: "Error connecting wallet",
            };
        } finally {
            pendingConnectWallet.value = false;
        }
    };

    const disconnectWallet = async () => {
        try {
            pendingDisconnectWallet.value = true;

            await disconnect();

            console.log("Disconnected wallet");

            if (callbackDisconnectWallet.value) {
                callbackDisconnectWallet.value();
            }

            return {
                success: true,
            };
        } catch (error) {
            return {
                success: false,
                message: "Error disconnecting wallet",
            };
        } finally {
            pendingDisconnectWallet.value = false;
        }
    };

    const hideConnectWallet = () => {
        callbackConnectWallet.value = null;
        openConnectWallet.value = false;
    };

    const showConnectWallet = (callback?: Function) => {
        callbackConnectWallet.value = callback || null;
        openConnectWallet.value = true;
    };

    const hideDisconnectWallet = () => {
        callbackDisconnectWallet.value = null;
        openDisconnectWallet.value = false;
    };

    const showDisconnectWallet = (callback?: Function) => {
        callbackDisconnectWallet.value = callback || null;
        openDisconnectWallet.value = true;
    };

    const payForQuotes = async (payments: [[string, string, string]]): Promise<string[]> => {
        const MAX_PAYMENTS_PER_TRANSACTION = 256;

        try {
            let totalAmount: bigint = payments.reduce((total, [_, __, amountStr]) => total + BigInt(amountStr), 0n);

            console.log("Payments:", payments);
            console.log("Total amount to pay:", totalAmount);

            let allowance = await getAllowance(wallet.value.address, paymentVaultContractAddress);

            if (allowance < totalAmount) {
                // approve contract to spend tokens
                await approveTokens(paymentVaultContractAddress, totalAmount);
            }

            let batches = [];

            for (let i = 0; i < payments.length; i += MAX_PAYMENTS_PER_TRANSACTION) {
                batches.push(payments.slice(i, i + MAX_PAYMENTS_PER_TRANSACTION));
            }

            let txHashes = [];

            for (const batch of batches) {
                let input = batch.map(([quoteHash, rewardsAddress, amountStr]) => [rewardsAddress, amountStr, quoteHash]);

                let txHash = await writeContract(wagmiAdapter.wagmiConfig, {
                    abi: paymentVaultAbi,
                    address: paymentVaultContractAddress,
                    functionName: "payForQuotes",
                    args: [input]
                });

                // wait for transaction
                let _receipt = await waitForTransactionReceipt(wagmiAdapter.wagmiConfig, {hash: txHash});

                txHashes.push(txHash);
            }

            return txHashes;
        } catch (error) {
            console.error("Error paying for quotes:", error);
            throw new Error("Failed to pay for quotes");
        }
    }

    const approveTokens = async (spenderAddress: string, approveAmount: bigint) => {
        try {
            let txHash = await writeContract(wagmiAdapter.wagmiConfig, {
                abi: tokenAbi,
                address: tokenContractAddress,
                functionName: "approve",
                args: [spenderAddress, approveAmount]
            });

            console.log("Approval transaction sent:", txHash);

            const receipt = await waitForTransactionReceipt(wagmiAdapter.wagmiConfig, {hash: txHash});

            console.log("Approval transaction receipt:", receipt);
        } catch (error) {
            console.error("Error approving to spend tokens:", error);
            throw new Error("Failed to approve token spend");
        }
    }


    const getAllowance = async (ownerAddress: string, spenderAddress: string): Promise<bigint> => {
        try {
            const result = await readContract(wagmiAdapter.wagmiConfig, {
                abi: tokenAbi,
                address: tokenContractAddress,
                functionName: "allowance",
                args: [ownerAddress, spenderAddress]
            });

            console.log("Approved amount for spender:", result);

            return BigInt(result);
        } catch (error) {
            console.error("Error fetching approved amount:", error);
            throw new Error("Failed to get approved amount");
        }
    };

    const getVaultKey = async (): Promise<string> => {
        return sign(VAULT_SECRET_KEY_SEED);
    }

    const sign = async (message: string): Promise<string> => {
        try {
            return await signMessage(wagmiAdapter.wagmiConfig, {message});
        } catch (error) {
            console.error("Error signing message:", error);
            throw new Error("Failed to sign message");
        }
    };

    // Return
    return {
        // State
        pendingConnectWallet,
        pendingDisconnectWallet,
        openConnectWallet,
        openDisconnectWallet,
        wallet,
        // Actions
        connectWallet,
        disconnectWallet,
        hideConnectWallet,
        showConnectWallet,
        hideDisconnectWallet,
        showDisconnectWallet,
        payForQuotes,
        approveTokens,
        getVaultKeySignature: getVaultKey,
        sign,
    };
});
