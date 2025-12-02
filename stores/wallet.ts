import {useAppKit, useAppKitAccount, useDisconnect} from "@reown/appkit/vue";
import {
    getBalance,
    getWalletClient,
    readContract,
    signMessage,
    waitForTransactionReceipt,
    writeContract
} from "@wagmi/core";
import {
    type Call,
    concat,
    createPublicClient,
    encodeFunctionData,
    formatUnits,
    type Hex,
    http,
    keccak256,
    toBytes,
    toHex,
    type WalletClient
} from "viem";
import {arbitrum} from "viem/chains";
import tokenAbi from "~/assets/abi/PaymentToken.json";
import paymentVaultAbi from "~/assets/abi/IPaymentVault.json";
import paymasterAbi from "~/assets/abi/AutonomiPaymaster.json";
import permitAbi from "~/assets/abi/Permit.json";
import {wagmiAdapter} from "~/config";
import {PAYMASTER_ADDRESS, PIMLICO_API_KEY} from "~/config/paymaster";
import {
    createPimlicoSmartAccountClient,
    getSmartAccount,
    type ToSafeSmartAccountReturnType
} from "~/utils/smartAccount";
import {signERC20Permit} from "~/utils/permit";
import debounce from "~/utils/debounce";
import {invoke} from "@tauri-apps/api/core";
import type {SmartAccountInfo, PaymasterCostEstimate, PaymasterCostEstimateOptions} from "~/types/paymaster";

const tokenContractAddress = "0xa78d8321B20c4Ef90eCd72f2588AA985A4BDb684";
const paymentVaultContractAddress = "0xB1b5219f8Aaa18037A2506626Dd0406a46f70BcC";
const VAULT_SECRET_KEY_SEED = "Massive Array of Internet Disks Secure Access For Everyone";
const MAX_PAYMENTS_PER_TRANSACTION = 256;

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
    const ethBalance = ref<string>('0');
    const antBalance = ref<string>('0');
    const balancesLoading = ref(false);
    const balanceRefreshInterval = ref<NodeJS.Timeout | null>(null);

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
        try {
            // Check if paymaster is enabled
            const appData = await invoke('app_data') as any;
            const usePaymaster = appData.use_paymaster ?? false;

            if (usePaymaster) {
                console.log("Using paymaster for payment");
                return await payForQuotesWithPaymaster(payments);
            } else {
                console.log("Using standard payment (requires ETH for gas)");
                return await payForQuotesStandard(payments);
            }
        } catch (error) {
            console.error("Error paying for quotes:", error);
            // User-friendly error message
            throw new Error("Something went wrong. Please try again.");
        }
    }

    const payForQuotesStandard = async (payments: [string, string, string][]): Promise<string[]> => {
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

            await new Promise(resolve => setTimeout(resolve, 1000));

            // wait for transaction
            let _receipt = await waitForTransactionReceipt(wagmiAdapter.wagmiConfig, {hash: txHash});

            txHashes.push(txHash);
        }

        // Refresh balances after payment
        refreshBalances();

        return txHashes;
    }

    const fundSmartAccountIfNeeded = async (
        smartAccount: ToSafeSmartAccountReturnType<"0.7">,
        smartAccountClient: any,
        publicClient: any,
        walletClient: WalletClient,
        requiredAmount: bigint
    ): Promise<void> => {
        // Check smart account ANT balance
        const smartAccountBalance = await readContract(wagmiAdapter.wagmiConfig, {
            abi: tokenAbi,
            address: tokenContractAddress,
            functionName: "balanceOf",
            args: [smartAccount.address]
        }) as bigint;

        console.log("Smart account ANT balance:", smartAccountBalance);

        // If smart account doesn't have enough ANT, we need to fund it first
        const needsTransfer = smartAccountBalance < requiredAmount;

        if (!needsTransfer) {
            console.log("Smart account has sufficient ANT - no funding needed");
            return;
        }

        const transferAmount = requiredAmount - smartAccountBalance;

        console.log("Smart account is missing ANT amount:", transferAmount);

        // Check EOA has enough ANT balance
        const eoaBalance = await readContract(wagmiAdapter.wagmiConfig, {
            abi: tokenAbi,
            address: tokenContractAddress,
            functionName: "balanceOf",
            args: [wallet.value.address]
        }) as bigint;

        console.log("EOA ANT balance:", eoaBalance);

        // STEP 1: First estimate gas cost with a signed permit for gas estimation
        console.log("Estimating gas for funding transaction...");

        // Sign permit for gas estimation (using transferAmount as initial estimate)
        const estimationPermitSignature = await signERC20Permit(
            walletClient,
            publicClient,
            tokenContractAddress as `0x${string}`,
            smartAccount.address,
            transferAmount
        );

        console.log("Permit signed successfully for gas estimation:", estimationPermitSignature);

        // Create dummy funding calls for gas estimation
        const dummyFundingCalls: Call[] = [
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: permitAbi,
                    functionName: "permit",
                    args: [
                        wallet.value.address as `0x${string}`,
                        smartAccount.address,
                        transferAmount,
                        estimationPermitSignature.deadline,
                        estimationPermitSignature.v,
                        estimationPermitSignature.r,
                        estimationPermitSignature.s,
                    ],
                }) as Hex,
            },
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: tokenAbi,
                    functionName: "transferFrom",
                    args: [wallet.value.address as `0x${string}`, smartAccount.address, transferAmount],
                }) as Hex,
            },
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: tokenAbi,
                    functionName: "approve",
                    args: [PAYMASTER_ADDRESS, transferAmount],
                }) as Hex,
            }
        ];

        // Estimate gas cost in ANT for funding transaction
        const fundingGasCostInANT = await estimateGasInAnt(
            smartAccountClient,
            publicClient,
            dummyFundingCalls
        );

        console.log("Funding gas cost in ANT:", fundingGasCostInANT);

        // STEP 2: Now calculate the actual amount we need to transfer (including gas cost)
        const actualTransferAmount = transferAmount + fundingGasCostInANT;

        console.log("Actual transfer amount (including gas):", actualTransferAmount);

        // Check EOA has enough for the actual transfer amount
        if (eoaBalance < actualTransferAmount) {
            throw new Error(`Insufficient ANT balance in EOA. Have: ${eoaBalance}, Need: ${actualTransferAmount}`);
        }

        // STEP 3: Sign permit with the ACTUAL transfer amount
        const permitSignature = await signERC20Permit(
            walletClient,
            publicClient,
            tokenContractAddress as `0x${string}`,
            smartAccount.address,
            actualTransferAmount
        );

        console.log("Permit signed successfully for actual amount");

        // STEP 4: Create the real funding calls with correct amounts
        const fundingCalls: Call[] = [
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: permitAbi,
                    functionName: "permit",
                    args: [
                        wallet.value.address as `0x${string}`,
                        smartAccount.address,
                        actualTransferAmount,
                        permitSignature.deadline,
                        permitSignature.v,
                        permitSignature.r,
                        permitSignature.s,
                    ],
                }) as Hex,
            },
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: tokenAbi,
                    functionName: "transferFrom",
                    args: [wallet.value.address as `0x${string}`, smartAccount.address, actualTransferAmount],
                }) as Hex,
            },
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: tokenAbi,
                    functionName: "approve",
                    args: [PAYMASTER_ADDRESS, actualTransferAmount],
                }) as Hex,
            }
        ];

        console.log("Sending funding transaction...");

        // Send funding transaction
        const fundingTxHash = await smartAccountClient.sendTransaction({
            calls: fundingCalls
        });

        console.log("Funding transaction hash:", fundingTxHash);

        // Wait a bit for the transaction to be processed
        await new Promise(resolve => setTimeout(resolve, 2000));

        console.log("Smart account funded successfully");
    };

    const estimateGasInAnt = async (
        smartAccountClient: any,
        publicClient: any,
        calls: Call[]
    ): Promise<bigint> => {
        console.log('[estimateGasInAnt] Starting gas estimation...');

        try {
            // Estimate gas for the user operation
            console.log('[estimateGasInAnt] Calling estimateUserOperationGas...');
            const gasEstimate = await smartAccountClient.estimateUserOperationGas({
                calls
            });

            console.log("[estimateGasInAnt] Gas estimate:", gasEstimate);

            console.log('[estimateGasInAnt] Getting gas price...');
            const gasPrice = await publicClient.getGasPrice();

            console.log("[estimateGasInAnt] Gas price:", gasPrice);

            const totalGasLimit =
                gasEstimate.callGasLimit +
                gasEstimate.verificationGasLimit +
                gasEstimate.preVerificationGas +
                (gasEstimate.paymasterVerificationGasLimit ?? 0n) +
                (gasEstimate.paymasterPostOpGasLimit ?? 0n);

            const maxCost = gasPrice * totalGasLimit;

            console.log("[estimateGasInAnt] Max cost in ETH:", maxCost);

            // Calculate gas cost in ANT tokens
            console.log('[estimateGasInAnt] Converting gas cost to ANT...');
            const gasCostInAnt = await publicClient.readContract({
                address: PAYMASTER_ADDRESS,
                abi: paymasterAbi,
                functionName: "calculateGasCostInANT",
                args: [maxCost]
            }) as bigint;

            console.log('[estimateGasInAnt] Gas cost in ANT:', gasCostInAnt);
            return gasCostInAnt;
        } catch (err) {
            console.error('[estimateGasInAnt] Error during gas estimation:', err);
            throw err;
        }
    };

    const payForQuotesWithPaymaster = async (payments: [string, string, string][]): Promise<string[]> => {
        // Get Pimlico API key - use hardcoded constant or fallback to env var
        const config = useRuntimeConfig();
        const pimlicoApiKey = PIMLICO_API_KEY || config.public.pimlicoApiKey;

        if (!pimlicoApiKey) {
            throw new Error("Pimlico API key not configured");
        }

        // Get wallet client for signing
        const walletClient = await getWalletClient(wagmiAdapter.wagmiConfig);
        if (!walletClient) {
            console.error("Wallet client not available");
            throw new Error("Something went wrong. Please try again.");
        }

        // Create smart account
        const smartAccount = await getSmartAccount(walletClient, arbitrum);
        console.log("Smart account address:", smartAccount.address);

        // Create smart account client with paymaster
        const smartAccountClient = await createPimlicoSmartAccountClient(
            smartAccount,
            pimlicoApiKey,
            PAYMASTER_ADDRESS,
            arbitrum
        );

        const publicClient = createPublicClient({
            chain: arbitrum,
            transport: http(),
        });

        let totalAmount: bigint = payments.reduce((total, [_, __, amountStr]) => total + BigInt(amountStr), 0n);

        console.log("Payments:", payments);
        console.log("Total amount to pay:", totalAmount);

        // STEP 1: Create payment batches
        let batches = [];

        for (let i = 0; i < payments.length; i += MAX_PAYMENTS_PER_TRANSACTION) {
            batches.push(payments.slice(i, i + MAX_PAYMENTS_PER_TRANSACTION));
        }

        console.log("Processing", batches.length, "payment batches");

        // STEP 2: Estimate total gas costs for ALL payment batches
        console.log("Estimating gas costs for all payment batches...");
        let totalPaymentGasCost = 0n;

        for (let batchIndex = 0; batchIndex < batches.length; batchIndex++) {
            const batch = batches[batchIndex];
            const batchAmount = batch.reduce((total, [_, __, amountStr]) => total + BigInt(amountStr), 0n);
            let input = batch.map(([quoteHash, rewardsAddress, amountStr]) => [rewardsAddress, amountStr, quoteHash]);

            // Create dummy payment calls for gas estimation
            const dummyPaymentCalls: Call[] = [
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: tokenAbi,
                        functionName: "approve",
                        args: [paymentVaultContractAddress, batchAmount],
                    }) as Hex,
                },
                {
                    to: paymentVaultContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: paymentVaultAbi,
                        functionName: "payForQuotes",
                        args: [input]
                    }) as Hex,
                },
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: tokenAbi,
                        functionName: "approve",
                        args: [PAYMASTER_ADDRESS, batchAmount],
                    }) as Hex,
                }
            ];

            const batchGasCost = await estimateGasInAnt(
                smartAccountClient,
                publicClient,
                dummyPaymentCalls
            );

            console.log(`Batch ${batchIndex} estimated gas cost: ${batchGasCost}`);
            totalPaymentGasCost += batchGasCost;
        }

        console.log("Total estimated payment gas cost:", totalPaymentGasCost);

        // STEP 3: Calculate required amount including gas costs
        const requiredAmount = totalAmount + totalPaymentGasCost;

        console.log("Required amount in smart account (payment + gas):", requiredAmount);

        // STEP 4: Fund smart account if needed
        await fundSmartAccountIfNeeded(
            smartAccount,
            smartAccountClient,
            publicClient,
            walletClient,
            requiredAmount
        );

        // STEP 5: Execute payments
        let txHashes = [];

        for (let batchIndex = 0; batchIndex < batches.length; batchIndex++) {
            const batch = batches[batchIndex];

            // Calculate amount for THIS batch only
            const batchAmount = batch.reduce((total, [_, __, amountStr]) => total + BigInt(amountStr), 0n);

            console.log(`Processing batch ${batchIndex} with amount ${batchAmount}`);

            let input = batch.map(([quoteHash, rewardsAddress, amountStr]) => [rewardsAddress, amountStr, quoteHash]);

            // Encode approve vault call
            const approveVaultCallData = encodeFunctionData({
                abi: tokenAbi,
                functionName: "approve",
                args: [paymentVaultContractAddress, batchAmount],
            });

            // Encode payForQuotes call
            const payForQuotesCalldata = encodeFunctionData({
                abi: paymentVaultAbi,
                functionName: "payForQuotes",
                args: [input]
            });

            const approveCallData = encodeFunctionData({
                abi: tokenAbi,
                functionName: "approve",
                args: [PAYMASTER_ADDRESS, batchAmount],
            });

            // Create payment calls (approve vault + payForQuotes + approve paymaster placeholder)
            let paymentCalls: Call[] = [
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: approveVaultCallData as Hex,
                },
                {
                    to: paymentVaultContractAddress as `0x${string}`,
                    value: 0n,
                    data: payForQuotesCalldata as Hex,
                },
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: approveCallData as Hex,
                }
            ];

            console.log(`Estimating gas for payment batch ${batchIndex}...`);

            // Estimate gas cost in ANT for payment transaction
            const paymentGasCostInANT = await estimateGasInAnt(
                smartAccountClient,
                publicClient,
                paymentCalls
            );

            console.log("Payment gas cost in ANT:", paymentGasCostInANT);

            // Update the approve paymaster call with actual gas cost
            paymentCalls[2].data = encodeFunctionData({
                abi: tokenAbi,
                functionName: "approve",
                args: [PAYMASTER_ADDRESS, paymentGasCostInANT],
            }) as Hex;

            console.log(`Sending payment batch ${batchIndex}...`);

            // Send payment transaction
            const txHash = await smartAccountClient.sendTransaction({
                calls: paymentCalls
            });

            console.log("Payment transaction hash:", txHash);
            txHashes.push(txHash);

            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        // Refresh balances after payment
        refreshBalances();

        return txHashes;
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
            try {
                const signature = await sign(keccak256(ethSignedMessageHash));
                cachedVaultKeySignature.value = signature.slice(0, -2); // Remove recovery ID
            } finally {
                pendingMessageSignature.value = false;
            }
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

    const hasVaultSignature = (): boolean => {
        // Check for development environment variable first
        const config = useRuntimeConfig();
        const devVaultSignature = config.public.devVaultSignature;

        if (devVaultSignature) {
            return true;
        }

        // Check for cached signature
        return !!cachedVaultKeySignature.value;
    };

    const fetchEthBalance = async (): Promise<string> => {
        if (!wallet.value.address) {
            return '0';
        }

        try {
            const balance = await getBalance(wagmiAdapter.wagmiConfig, {
                address: wallet.value.address,
            });

            return formatUnits(balance.value, balance.decimals);
        } catch (error) {
            console.error("Error fetching ETH balance:", error);
            return '0';
        }
    };

    const fetchAntBalance = async (): Promise<string> => {
        if (!wallet.value.address) {
            return '0';
        }

        try {
            const balance = await readContract(wagmiAdapter.wagmiConfig, {
                abi: tokenAbi,
                address: tokenContractAddress,
                functionName: "balanceOf",
                args: [wallet.value.address]
            });

            const decimals = await readContract(wagmiAdapter.wagmiConfig, {
                abi: tokenAbi,
                address: tokenContractAddress,
                functionName: "decimals",
                args: []
            });

            return formatUnits(BigInt(balance), Number(decimals));
        } catch (error) {
            console.error("Error fetching ANT balance:", error);
            return '0';
        }
    };

    const refreshBalances = async (showLoading: boolean = false): Promise<void> => {
        if (!wallet.value.address) {
            ethBalance.value = '0';
            antBalance.value = '0';
            return;
        }

        try {
            if (showLoading) {
                balancesLoading.value = true;
            }
            const [ethBal, antBal] = await Promise.all([
                fetchEthBalance(),
                fetchAntBalance()
            ]);

            ethBalance.value = ethBal;
            antBalance.value = antBal;
        } catch (error) {
            console.error("Error refreshing balances:", error);
        } finally {
            if (showLoading) {
                balancesLoading.value = false;
            }
        }
    };

    // Watch for wallet address changes and refresh balances
    watch(() => wallet.value.address, (newAddress, oldAddress) => {
        if (newAddress) {
            // Show loading only on initial connection (when oldAddress was null)
            refreshBalances(!oldAddress);
            // Set up interval to refresh balances every minute
            if (balanceRefreshInterval.value) {
                clearInterval(balanceRefreshInterval.value);
            }
            balanceRefreshInterval.value = setInterval(() => {
                refreshBalances(false); // No loading indicator for periodic refreshes
            }, 60000); // 60 seconds
        } else {
            ethBalance.value = '0';
            antBalance.value = '0';
            // Clear interval when wallet disconnects
            if (balanceRefreshInterval.value) {
                clearInterval(balanceRefreshInterval.value);
                balanceRefreshInterval.value = null;
            }
        }
    }, {immediate: true});

    // Paymaster guided flow helper functions
    const getSmartAccountInfo = async (): Promise<SmartAccountInfo> => {
        // Get Pimlico API key - use hardcoded constant or fallback to env var
        const config = useRuntimeConfig();
        const pimlicoApiKey = PIMLICO_API_KEY || config.public.pimlicoApiKey;

        if (!pimlicoApiKey) {
            throw new Error("Pimlico API key not configured");
        }

        const walletClient = await getWalletClient(wagmiAdapter.wagmiConfig);
        if (!walletClient) {
            console.error("Wallet client not available");
            throw new Error("Something went wrong. Please try again.");
        }

        const smartAccount = await getSmartAccount(walletClient, arbitrum);

        // Check smart account ANT balance
        const antBalance = await readContract(wagmiAdapter.wagmiConfig, {
            abi: tokenAbi,
            address: tokenContractAddress,
            functionName: "balanceOf",
            args: [smartAccount.address]
        }) as bigint;

        // Check if smart account is deployed by checking if it has any code
        const publicClient = createPublicClient({
            chain: arbitrum,
            transport: http(),
        });

        const code = await publicClient.getBytecode({address: smartAccount.address as `0x${string}`});
        const exists = !!code && code !== '0x';

        return {
            address: smartAccount.address,
            antBalance,
            exists
        };
    };

    const estimatePaymasterCosts = async (
        payments: [string, string, string][],
        options?: PaymasterCostEstimateOptions
    ): Promise<PaymasterCostEstimate> => {
        console.log('[estimatePaymasterCosts] Starting cost estimation...');

        // Get Pimlico API key - use hardcoded constant or fallback to env var
        const config = useRuntimeConfig();
        const pimlicoApiKey = PIMLICO_API_KEY || config.public.pimlicoApiKey;

        if (!pimlicoApiKey) {
            throw new Error("Pimlico API key not configured");
        }

        console.log('[estimatePaymasterCosts] Getting wallet client...');
        const walletClient = await getWalletClient(wagmiAdapter.wagmiConfig);
        if (!walletClient) {
            console.error("Wallet client not available");
            throw new Error("Something went wrong. Please try again.");
        }
        console.log('[estimatePaymasterCosts] Wallet client obtained');

        const smartAccount = await getSmartAccount(walletClient, arbitrum);
        const smartAccountClient = await createPimlicoSmartAccountClient(
            smartAccount,
            pimlicoApiKey,
            PAYMASTER_ADDRESS,
            arbitrum
        );

        const publicClient = createPublicClient({
            chain: arbitrum,
            transport: http(),
        });

        console.log('[estimatePaymasterCosts] Received payments:', payments);
        console.log('[estimatePaymasterCosts] First payment amount type:', typeof payments[0]?.[2]);
        console.log('[estimatePaymasterCosts] First payment amount value:', payments[0]?.[2]);

        // Calculate total payment amount - handle both string and bigint amounts
        const totalPaymentAmount = payments.reduce((total, [_, __, amountStr]) => {
            const amount = typeof amountStr === 'bigint' ? amountStr : BigInt(amountStr);
            console.log('[estimatePaymasterCosts] Processing payment amount:', amountStr, '-> BigInt:', amount);
            return total + amount;
        }, 0n);

        console.log('[estimatePaymasterCosts] Total payment amount (storage cost):', totalPaymentAmount);

        // Create payment batches
        const batches = [];
        for (let i = 0; i < payments.length; i += MAX_PAYMENTS_PER_TRANSACTION) {
            batches.push(payments.slice(i, i + MAX_PAYMENTS_PER_TRANSACTION));
        }

        // Estimate gas costs for all payment batches
        console.log('[estimatePaymasterCosts] Estimating gas for', batches.length, 'batches...');
        let totalPaymentGasCost = 0n;
        for (let batchIndex = 0; batchIndex < batches.length; batchIndex++) {
            console.log(`[estimatePaymasterCosts] Processing batch ${batchIndex + 1}/${batches.length}...`);
            const batch = batches[batchIndex];
            const batchAmount = batch.reduce((total, [_, __, amountStr]) => {
                const amount = typeof amountStr === 'bigint' ? amountStr : BigInt(amountStr);
                return total + amount;
            }, 0n);
            const input = batch.map(([quoteHash, rewardsAddress, amountStr]) => {
                const amount = typeof amountStr === 'bigint' ? amountStr.toString() : amountStr;
                return [rewardsAddress, amount, quoteHash];
            });

            console.log(`[estimatePaymasterCosts] Batch ${batchIndex} amount:`, batchAmount);

            const dummyPaymentCalls: Call[] = [
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: tokenAbi,
                        functionName: "approve",
                        args: [paymentVaultContractAddress, batchAmount],
                    }) as Hex,
                },
                {
                    to: paymentVaultContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: paymentVaultAbi,
                        functionName: "payForQuotes",
                        args: [input]
                    }) as Hex,
                },
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: tokenAbi,
                        functionName: "approve",
                        args: [PAYMASTER_ADDRESS, batchAmount],
                    }) as Hex,
                }
            ];

            console.log(`[estimatePaymasterCosts] Estimating gas for batch ${batchIndex}...`);
            try {
                const batchGasCost = await estimateGasInAnt(smartAccountClient, publicClient, dummyPaymentCalls);
                console.log(`[estimatePaymasterCosts] Batch ${batchIndex} gas cost:`, batchGasCost);
                totalPaymentGasCost += batchGasCost;
            } catch (err) {
                console.error(`[estimatePaymasterCosts] Error estimating gas for batch ${batchIndex}:`, err);
                throw err;
            }
        }
        console.log('[estimatePaymasterCosts] Total payment gas cost:', totalPaymentGasCost);

        // Get current smart account balance - use cached value if provided
        let currentSmartAccountBalance: bigint;
        if (options?.currentSmartAccountBalance !== undefined) {
            console.log('[estimatePaymasterCosts] Using cached smart account balance:', options.currentSmartAccountBalance);
            currentSmartAccountBalance = options.currentSmartAccountBalance;
        } else {
            console.log('[estimatePaymasterCosts] Reading smart account balance...');
            try {
                const balancePromise = readContract(wagmiAdapter.wagmiConfig, {
                    abi: tokenAbi,
                    address: tokenContractAddress,
                    functionName: "balanceOf",
                    args: [smartAccount.address]
                });

                // Add 10 second timeout
                const timeoutPromise = new Promise<never>((_, reject) =>
                    setTimeout(() => reject(new Error('Timeout reading smart account balance')), 10000)
                );

                currentSmartAccountBalance = await Promise.race([balancePromise, timeoutPromise]) as bigint;
                console.log('[estimatePaymasterCosts] Smart account balance:', currentSmartAccountBalance);
            } catch (err) {
                console.error('[estimatePaymasterCosts] Error reading smart account balance:', err);
                throw new Error('Failed to read smart account balance. Please try again.');
            }
        }

        // Calculate required amount
        const requiredInSmartAccount = totalPaymentAmount + totalPaymentGasCost;

        // Funding is required if smart account balance is less than what's needed
        const fundingRequired = currentSmartAccountBalance < requiredInSmartAccount;
        const baseTransferAmount = fundingRequired ? requiredInSmartAccount - currentSmartAccountBalance : 0n;

        // If funding is required, estimate the funding gas cost
        let estimatedFundingGasCost = 0n;
        if (fundingRequired && baseTransferAmount > 0n) {
            console.log('[estimatePaymasterCosts] Funding required, estimating funding gas cost...');
            console.log('[estimatePaymasterCosts] Base transfer amount:', baseTransferAmount);
            // We need to sign a permit for estimation
            console.log('[estimatePaymasterCosts] Signing permit for funding gas estimation...');
            const estimationPermitSignature = await signERC20Permit(
                walletClient,
                publicClient,
                tokenContractAddress as `0x${string}`,
                smartAccount.address,
                baseTransferAmount
            );
            console.log('[estimatePaymasterCosts] Permit signed for funding estimation');

            const dummyFundingCalls: Call[] = [
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: permitAbi,
                        functionName: "permit",
                        args: [
                            wallet.value.address as `0x${string}`,
                            smartAccount.address,
                            baseTransferAmount,
                            estimationPermitSignature.deadline,
                            estimationPermitSignature.v,
                            estimationPermitSignature.r,
                            estimationPermitSignature.s,
                        ],
                    }) as Hex,
                },
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: tokenAbi,
                        functionName: "transferFrom",
                        args: [wallet.value.address as `0x${string}`, smartAccount.address, baseTransferAmount],
                    }) as Hex,
                },
                {
                    to: tokenContractAddress as `0x${string}`,
                    value: 0n,
                    data: encodeFunctionData({
                        abi: tokenAbi,
                        functionName: "approve",
                        args: [PAYMASTER_ADDRESS, baseTransferAmount],
                    }) as Hex,
                }
            ];

            estimatedFundingGasCost = await estimateGasInAnt(smartAccountClient, publicClient, dummyFundingCalls);
        }

        // The actual funding amount includes the gas cost for the funding transaction itself
        const fundingAmount = fundingRequired ? baseTransferAmount + estimatedFundingGasCost : 0n;
        const totalGasCost = totalPaymentGasCost + estimatedFundingGasCost;

        console.log('[estimatePaymasterCosts] === COST SUMMARY ===');
        console.log('[estimatePaymasterCosts] Total Payment Amount (storage):', totalPaymentAmount);
        console.log('[estimatePaymasterCosts] Payment Batch Count:', batches.length);
        console.log('[estimatePaymasterCosts] Estimated Payment Gas Cost:', totalPaymentGasCost);
        console.log('[estimatePaymasterCosts] Estimated Funding Gas Cost:', estimatedFundingGasCost);
        console.log('[estimatePaymasterCosts] Total Gas Cost:', totalGasCost);
        console.log('[estimatePaymasterCosts] Required in Smart Account:', requiredInSmartAccount);
        console.log('[estimatePaymasterCosts] Current Smart Account Balance:', currentSmartAccountBalance);
        console.log('[estimatePaymasterCosts] Funding Required:', fundingRequired);
        console.log('[estimatePaymasterCosts] Funding Amount:', fundingAmount);
        console.log('[estimatePaymasterCosts] ====================');

        return {
            totalPaymentAmount,
            paymentBatchCount: batches.length,
            estimatedPaymentGasCost: totalPaymentGasCost,
            estimatedFundingGasCost,
            totalGasCost,
            requiredInSmartAccount,
            currentSmartAccountBalance,
            fundingRequired,
            fundingAmount
        };
    };

    const fundSmartAccount = async (amount: bigint): Promise<void> => {
        // Get Pimlico API key - use hardcoded constant or fallback to env var
        const config = useRuntimeConfig();
        const pimlicoApiKey = PIMLICO_API_KEY || config.public.pimlicoApiKey;

        if (!pimlicoApiKey) {
            throw new Error("Pimlico API key not configured");
        }

        const walletClient = await getWalletClient(wagmiAdapter.wagmiConfig);
        if (!walletClient) {
            console.error("Wallet client not available");
            throw new Error("Something went wrong. Please try again.");
        }

        const smartAccount = await getSmartAccount(walletClient, arbitrum);
        const smartAccountClient = await createPimlicoSmartAccountClient(
            smartAccount,
            pimlicoApiKey,
            PAYMASTER_ADDRESS,
            arbitrum
        );

        const publicClient = createPublicClient({
            chain: arbitrum,
            transport: http(),
        });

        // Check EOA has enough ANT balance
        const eoaBalance = await readContract(wagmiAdapter.wagmiConfig, {
            abi: tokenAbi,
            address: tokenContractAddress,
            functionName: "balanceOf",
            args: [wallet.value.address]
        }) as bigint;

        if (eoaBalance < amount) {
            throw new Error(`Insufficient ANT balance in wallet. Have: ${eoaBalance}, Need: ${amount}`);
        }

        // Sign permit for the transfer
        const permitSignature = await signERC20Permit(
            walletClient,
            publicClient,
            tokenContractAddress as `0x${string}`,
            smartAccount.address,
            amount
        );

        // Create funding calls
        const fundingCalls: Call[] = [
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: permitAbi,
                    functionName: "permit",
                    args: [
                        wallet.value.address as `0x${string}`,
                        smartAccount.address,
                        amount,
                        permitSignature.deadline,
                        permitSignature.v,
                        permitSignature.r,
                        permitSignature.s,
                    ],
                }) as Hex,
            },
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: tokenAbi,
                    functionName: "transferFrom",
                    args: [wallet.value.address as `0x${string}`, smartAccount.address, amount],
                }) as Hex,
            },
            {
                to: tokenContractAddress as `0x${string}`,
                value: 0n,
                data: encodeFunctionData({
                    abi: tokenAbi,
                    functionName: "approve",
                    args: [PAYMASTER_ADDRESS, amount],
                }) as Hex,
            }
        ];

        console.log("Funding smart account with amount:", amount);

        // Send funding transaction
        const fundingTxHash = await smartAccountClient.sendTransaction({
            calls: fundingCalls
        });

        console.log("Funding transaction hash:", fundingTxHash);

        // Wait for the transaction to be processed
        await new Promise(resolve => setTimeout(resolve, 3000));

        // Refresh balances
        refreshBalances();

        console.log("Smart account funded successfully");
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
        ethBalance,
        antBalance,
        balancesLoading,
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
        hasVaultSignature,
        fetchEthBalance,
        fetchAntBalance,
        refreshBalances,
        // Paymaster guided flow
        getSmartAccountInfo,
        estimatePaymasterCosts,
        fundSmartAccount,
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
