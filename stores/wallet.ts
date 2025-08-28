import {useAppKit, useAppKitAccount, useDisconnect} from "@reown/appkit/vue";
import {readContract, signMessage, waitForTransactionReceipt, writeContract} from "@wagmi/core";
import {keccak256, concat, toHex, toBytes, hashMessage} from "viem";
import tokenAbi from "~/assets/abi/PaymentToken.json";
import paymentVaultAbi from "~/assets/abi/IPaymentVault.json";
import {wagmiAdapter} from "~/config";
import debounce from "~/utils/debounce";

const tokenContractAddress = "0xa78d8321B20c4Ef90eCd72f2588AA985A4BDb684";
const paymentVaultContractAddress = "0xB1b5219f8Aaa18037A2506626Dd0406a46f70BcC";
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

                const buttonWalletConnect = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connect-view')?.shadowRoot?.querySelector('w3m-wallet-login-list')?.shadowRoot?.querySelector('w3m-connector-list')?.shadowRoot?.querySelector('w3m-connect-walletconnect-widget');

                const modalHeaderText = walletModalShadow.querySelector('w3m-header')?.shadowRoot?.querySelector('wui-text');

                const buttonAllWallets = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connect-view')?.shadowRoot?.querySelector('w3m-wallet-login-list')?.shadowRoot?.querySelector('w3m-all-wallets-widget')?.shadowRoot?.querySelector('wui-list-wallet')?.shadowRoot?.querySelector('button')?.querySelector('wui-text');

                const buttonGetAWallet = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-what-is-a-wallet-view')?.shadowRoot?.querySelector('wui-button');

                const scanQrCodeText = walletModalShadow.querySelector('w3m-router')?.shadowRoot?.querySelector('w3m-connecting-wc-view')?.shadowRoot?.querySelector('w3m-connecting-wc-qrcode')?.shadowRoot?.querySelector('wui-text');

                let walletName = '';

                if (modalHeaderText) {

                    const connectWalletRegex = /Connect\s+Wallet/i;
                    const isConnectWallet = connectWalletRegex.test(modalHeaderText.innerHTML); // Connect Wallet is the header
                    const placeholderElement = modalHeaderText.parentElement?.querySelector('.autonomi-header');

                    if (isConnectWallet) {
                        if (!placeholderElement) {
                            // Add the updated connect mobile wallet element if it doesn't exist
                            modalHeaderText.parentElement?.insertAdjacentHTML('beforeend', '<span class="autonomi-header" style="font-size: 14px; font-weight: 600; color: #ffffff;">Connect Mobile Wallet</span>');
                        }

                        modalHeaderText.style.position = 'absolute';
                        modalHeaderText.style.left = '-9999px';
                    } else {
                        // Show header
                        modalHeaderText.style.position = 'relative';
                        modalHeaderText.style.left = '0px';
                        placeholderElement?.remove();
                        // Set wallet name
                        walletName = modalHeaderText.textContent || '';
                    }
                }

                if (buttonAllWallets) {
                    buttonAllWallets.textContent = "All Mobile Wallets";
                }

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

                if (buttonWalletConnect) {
                    buttonWalletConnect.hidden = true;
                }

                if (buttonGetAWallet) {
                    buttonGetAWallet.hidden = true;
                }

                if (scanQrCodeText) {
                    scanQrCodeText.innerHTML = `<span style="display: block; text-align: center;">Download ${walletName} on your mobile device then scan this QR code.</span>`
                }
            } catch (error) {
                // TODO: Handle error
            }
        }, 75)

        const observer = new MutationObserver((mutationsList) => {
            mutationsList.forEach((mutation) => {
                hideModalElements()
            })
        })

        observer.observe(walletModalShadow, {attributes: true, childList: true, subtree: true});
    } catch (error) {
    }
}


export const useWalletStore = defineStore("wallet", () => {
    // State
    const pendingConnectWallet = ref(false);
    const pendingDisconnectWallet = ref(false);
    const pendingMessageSignature = ref(false);
    const openConnectWallet = ref(false);
    const openDisconnectWallet = ref(false);
    const callbackConnectWallet = ref<Function | null>(null);
    const callbackDisconnectWallet = ref<Function | null>(null);
    const cachedVaultKeySignature = ref<string>();

    const wallet = useAppKitAccount();
    const {open} = useAppKit();
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

            // Clear cached vault key signature on disconnect
            cachedVaultKeySignature.value = undefined;

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

    const payForQuotes = async (payments: [string, string, string][]): Promise<string[]> => {
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
        // Check for development environment variable first
        const config = useRuntimeConfig();
        const devVaultSignature = config.public.devVaultSignature;

        console.log("DEBUG: Runtime config devVaultSignature:", devVaultSignature);
        console.log("DEBUG: devVaultSignature type:", typeof devVaultSignature);
        console.log("DEBUG: devVaultSignature length:", devVaultSignature?.length);

        if (devVaultSignature) {
            console.log("Using development vault key signature from ENV");
            return devVaultSignature;
        }

        if (!cachedVaultKeySignature.value) {
            const hex = toHex(VAULT_SECRET_KEY_SEED);
            const ethSignedMessageHash = toEthSignedMessageHash(hex);
            pendingMessageSignature.value = true;
            const signature = await sign(keccak256(ethSignedMessageHash));
            cachedVaultKeySignature.value = signature.slice(0, -2); // Remove recovery ID
        }

        console.log("Cached vault key signature:", cachedVaultKeySignature.value);

        return cachedVaultKeySignature.value;
    }

    const sign = async (hex: `0x${string}`): Promise<string> => {
        console.log("Signing message:", hex);

        try {
            return await signMessage(wagmiAdapter.wagmiConfig, {
                message: {raw: hex},
            });
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
        pendingMessageSignature,
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

function toEthSignedMessageHash(message) {
    // First hash the input message (must be bytes or hex string)
    const messageHash = keccak256(message); // This gives a hex string

    const prefix = '\x19Ethereum Signed Message:\n32';
    const prefixBytes = toBytes(prefix);              // Converts to Uint8Array
    const messageHashBytes = toBytes(messageHash);    // Converts hash hex to bytes

    const ethMessage = concat([prefixBytes, messageHashBytes]);

    return keccak256(ethMessage);
}
