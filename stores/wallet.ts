import {useAppKit, useAppKitAccount, useDisconnect} from "@reown/appkit/vue";
import {readContract, signMessage, waitForTransactionReceipt, writeContract} from "@wagmi/core";
import tokenAbi from "~/assets/abi/PaymentToken.json";
import paymentVaultAbi from "~/assets/abi/IPaymentVault.json";
import {wagmiAdapter} from "~/config";
import debounce from "~/utils/debounce";

const tokenContractAddress = "0xBE1802c27C324a28aeBcd7eeC7D734246C807194";
const paymentVaultContractAddress = "0x993C7739f50899A997fEF20860554b8a28113634";
const VAULT_SECRET_KEY_SEED = "Massive Array of Internet Disks Secure Access For Everyone";

let isSetWalletModalListener = false;

const handleObserveHideModalElements = (walletModal: HTMLElement) => {
    try {
        const walletModalShadow = walletModal.shadowRoot as ShadowRoot;

        const hideModalElements = debounce(() => {
            try {
                const mobileTabHeader = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connecting-wc-view')?.shadowRoot?.querySelector('w3m-connecting-header');
                const copyLink = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connecting-wc-view')?.shadowRoot?.querySelector('w3m-connecting-wc-qrcode')?.shadowRoot?.querySelector('wui-link');
                const mobileDownloadLinks = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connecting-wc-view')?.shadowRoot?.querySelector('w3m-connecting-wc-qrcode')?.shadowRoot?.querySelector('w3m-mobile-download-links')?.shadowRoot?.querySelector('wui-cta-button');
                const getStartedLink = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connect-view')?.shadowRoot?.querySelector('w3m-wallet-guide');

                if (mobileTabHeader) {
                    mobileTabHeader.hidden = true;
                }

                if (copyLink) {
                    copyLink.hidden = true;
                }

                if (mobileDownloadLinks) {
                    mobileDownloadLinks.hidden = true;
                }

                if (getStartedLink) {
                    getStartedLink.hidden = true;
                }
            }
            catch (error) {
                // TODO: Handle error
            }
        }, 50)

        const observer = new MutationObserver((mutationsList) => {
            mutationsList.forEach((mutation) => {
                hideModalElements()
            })
        })

        observer.observe(walletModalShadow, { attributes: true, childList: true, subtree: true });
    }
    catch (error) {
    }
}


export const useWalletStore = defineStore("wallet", () => {
    // State
    const pendingConnectWallet = ref(false);
    const pendingDisconnectWallet = ref(false);
    const openConnectWallet = ref(false);
    const openDisconnectWallet = ref(false);
    const callbackConnectWallet = ref<Function | null>(null);
    const callbackDisconnectWallet = ref<Function | null>(null);
    const cachedVaultKey = ref<string>();

    const wallet = useAppKitAccount();
    const { open } = useAppKit();
    const {disconnect} = useDisconnect();

    const connectWallet = async () => {
        try {
            pendingConnectWallet.value = true;

            const connectResponse = await open();
            
            // Get wallet modal reference
            const walletModal = document.querySelector('w3m-modal') as HTMLElement | null;

            // Add observer
            if (!isSetWalletModalListener) {
                walletModal && handleObserveHideModalElements(walletModal)
                isSetWalletModalListener = true
            }
            
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

    const getVaultKeySignature = async (): Promise<string> => {
        if (!cachedVaultKey.value) {
            cachedVaultKey.value = await sign(VAULT_SECRET_KEY_SEED);
        }

        return cachedVaultKey.value;
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
        callbackConnectWallet,
        connectWallet,
        disconnectWallet,
        hideConnectWallet,
        showConnectWallet,
        hideDisconnectWallet,
        showDisconnectWallet,
        payForQuotes,
        approveTokens,
        getVaultKeySignature,
        sign,
    };
});
