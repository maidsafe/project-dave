<script lang="ts" setup>
import {storeToRefs} from "pinia";
import {useWalletStore} from "~/stores/wallet";
import type {SmartAccountInfo, PaymasterCostEstimate, PaymasterFlowStep} from "~/types/paymaster";
import {formatUnits} from "viem";

interface Props {
  visible: boolean;
  payments: [string, string, string][];
}

const props = defineProps<Props>();
const emit = defineEmits(['close', 'proceed-with-paymaster', 'cancel']);

const walletStore = useWalletStore();
const {wallet, antBalance} = storeToRefs(walletStore);

// Flow state
const currentStep = ref<PaymasterFlowStep>('checking-smart-account');
const smartAccount = ref<SmartAccountInfo | null>(null);
const costEstimate = ref<PaymasterCostEstimate | null>(null);
const userFundingAmount = ref<string>('');
const error = ref<string | null>(null);
const isProcessing = ref(false);
const signingMessage = ref<string>('');

// Computed
const hasSmartAccount = computed(() => smartAccount.value?.exists === true);

const needsFunding = computed(() => {
  if (!costEstimate.value) return false;
  return costEstimate.value.fundingRequired;
});

const totalAntNeeded = computed(() => {
  if (!costEstimate.value) return 0n;
  return costEstimate.value.totalPaymentAmount + costEstimate.value.totalGasCost;
});

const formattedANT = (attoAmount: bigint): string => {
  try {
    const formatted = formatUnits(attoAmount, 18);
    // Remove trailing zeros
    const parts = formatted.split('.');
    if (parts.length === 2) {
      const decimals = parts[1].replace(/0+$/, '');
      if (decimals.length === 0) {
        return parts[0];
      }
      // Show up to 6 decimal places
      return `${parts[0]}.${decimals.substring(0, 6)}`;
    }
    return formatted;
  } catch (error) {
    console.error('Error formatting ANT amount:', error);
    return '0';
  }
};

// Initialize flow
const initializeFlow = async () => {
  try {
    error.value = null;
    isProcessing.value = true;
    currentStep.value = 'checking-smart-account';

    console.log('[PaymasterGuide] Initializing flow with payments:', props.payments);

    // Step 1: Check smart account
    smartAccount.value = await walletStore.getSmartAccountInfo();
    console.log('[PaymasterGuide] Smart account info:', smartAccount.value);

    // Step 2: If smart account doesn't exist, skip cost estimation
    if (!smartAccount.value.exists) {
      // Smart account doesn't exist - needs to be created/funded for first time
      // Suggest 1 ANT as a starting amount (user can change it)
      console.log('[PaymasterGuide] Smart account does not exist, showing setup');
      userFundingAmount.value = '1';
      currentStep.value = 'smart-account-setup';
      return;
    }

    // Step 3: Smart account exists, estimate costs
    console.log('[PaymasterGuide] Smart account exists, estimating costs...');
    currentStep.value = 'cost-estimation';

    if (!props.payments || props.payments.length === 0) {
      throw new Error('No payment data available');
    }

    // Show signing message
    signingMessage.value = 'Estimating gas costs requires signing a permit. Please approve in your wallet.';

    // Pass the smart account balance we already fetched to avoid redundant RPC calls
    costEstimate.value = await walletStore.estimatePaymasterCosts(props.payments, {
      currentSmartAccountBalance: smartAccount.value.antBalance
    });
    console.log('[PaymasterGuide] Cost estimate:', costEstimate.value);

    // Clear signing message
    signingMessage.value = '';

    // Step 4: Determine next step based on funding requirements
    if (costEstimate.value.fundingRequired) {
      // Smart account exists but needs more funding - use calculated minimum
      console.log('[PaymasterGuide] Additional funding required');
      userFundingAmount.value = formattedANT(costEstimate.value.fundingAmount);
      currentStep.value = 'smart-account-setup';
    } else {
      // All good - smart account exists and has sufficient funds
      console.log('[PaymasterGuide] Smart account has sufficient funds, ready to proceed');
      currentStep.value = 'executing-payments';
    }
  } catch (err: any) {
    console.error('[PaymasterGuide] Error initializing flow:', err);
    // Show user-friendly message, log raw error to console
    error.value = 'Something went wrong. Please try again.';
    currentStep.value = 'error';
  } finally {
    isProcessing.value = false;
  }
};

// Watch for visibility changes
watch(() => props.visible, (visible) => {
  if (visible) {
    initializeFlow();
  } else {
    // Reset state when dialog closes
    currentStep.value = 'checking-smart-account';
    smartAccount.value = null;
    costEstimate.value = null;
    userFundingAmount.value = '';
    error.value = null;
  }
}, {immediate: true});

// Actions
const handleProceed = async () => {
  // If smart account doesn't exist, we need to create/fund it first
  if (!smartAccount.value?.exists) {
    await createAndFundSmartAccount();
  } else if (currentStep.value === 'smart-account-setup') {
    // Smart account exists but needs more funding
    await createAndFundSmartAccount();
  } else {
    // Smart account exists and has sufficient funds, proceed with payment
    // The actual payment will happen in the parent component
    emit('proceed-with-paymaster', {
      smartAccount: smartAccount.value,
      costEstimate: costEstimate.value,
      userFundingAmount: userFundingAmount.value ? BigInt(Math.floor(parseFloat(userFundingAmount.value) * 1e18)) : 0n
    });
  }
};

const createAndFundSmartAccount = async () => {
  try {
    isProcessing.value = true;
    currentStep.value = 'funding-smart-account';

    const fundingAmountBigInt = BigInt(Math.floor(parseFloat(userFundingAmount.value) * 1e18));

    // Show signing message for funding
    signingMessage.value = `Funding your smart account with ${userFundingAmount.value} ANT. Please sign the permit in your wallet.`;

    // Call wallet store to fund the smart account
    await walletStore.fundSmartAccount(fundingAmountBigInt);

    // Clear signing message
    signingMessage.value = '';

    // After successful funding, re-check smart account status
    smartAccount.value = await walletStore.getSmartAccountInfo();

    // Now estimate costs with the funded smart account
    currentStep.value = 'cost-estimation';
    console.log('[PaymasterGuide] After funding, estimating costs with payments:', props.payments);
    console.log('[PaymasterGuide] Payments array length:', props.payments.length);

    // Show signing message for cost estimation
    signingMessage.value = 'Estimating costs requires signing a permit. Please approve in your wallet.';

    // Pass the updated smart account balance to avoid redundant RPC calls
    costEstimate.value = await walletStore.estimatePaymasterCosts(props.payments, {
      currentSmartAccountBalance: smartAccount.value.antBalance
    });

    // Clear signing message
    signingMessage.value = '';

    // Check if we need more funding or if we're ready
    if (costEstimate.value.fundingRequired) {
      userFundingAmount.value = formattedANT(costEstimate.value.fundingAmount);
      currentStep.value = 'smart-account-setup';
    } else {
      currentStep.value = 'executing-payments';
    }
  } catch (err: any) {
    console.error('[PaymasterGuide] Error funding smart account:', err);
    // Show user-friendly message, log raw error to console
    error.value = 'Something went wrong. Please try again.';
    currentStep.value = 'error';
  } finally {
    isProcessing.value = false;
  }
};

const handleCancel = () => {
  emit('cancel');
};

const handleClose = () => {
  emit('close');
};

// Validation
const canProceed = computed(() => {
  if (currentStep.value === 'smart-account-setup') {
    if (!userFundingAmount.value || parseFloat(userFundingAmount.value) <= 0) {
      return false;
    }
    // Check user has enough ANT in their EOA
    const userAntBalance = BigInt(Math.floor(parseFloat(antBalance.value) * 1e18));
    const fundingAmountBigInt = BigInt(Math.floor(parseFloat(userFundingAmount.value) * 1e18));

    if (fundingAmountBigInt > userAntBalance) {
      return false;
    }

    // Only check minimum required if smart account already exists (we have cost estimates)
    if (smartAccount.value?.exists && costEstimate.value && fundingAmountBigInt < costEstimate.value.fundingAmount) {
      return false;
    }

    return true;
  }

  if (currentStep.value === 'executing-payments') {
    return true;
  }

  return false;
});

const insufficientBalance = computed(() => {
  if (!userFundingAmount.value) return false;
  const userAntBalance = BigInt(Math.floor(parseFloat(antBalance.value) * 1e18));
  const fundingAmountBigInt = BigInt(Math.floor(parseFloat(userFundingAmount.value) * 1e18));
  return fundingAmountBigInt > userAntBalance;
});

const belowMinimum = computed(() => {
  // Only check minimum if smart account exists (we have actual cost estimates)
  if (!smartAccount.value?.exists || !userFundingAmount.value || !costEstimate.value) return false;
  const fundingAmountBigInt = BigInt(Math.floor(parseFloat(userFundingAmount.value) * 1e18));
  return fundingAmountBigInt < costEstimate.value.fundingAmount;
});
</script>

<template>
  <Dialog
      :visible="props.visible"
      modal
      header="Gasless Payment"
      :style="{ width: '40rem' }"
      position="center"
      :draggable="false"
      pt:root:class="!border-0"
      pt:mask:class="backdrop-blur-sm"
      :closable="currentStep === 'error' || currentStep === 'checking-smart-account'"
      @update:visible="handleClose"
  >
    <div class="space-y-6">
      <!-- Loading: Checking Smart Account -->
      <div v-if="currentStep === 'checking-smart-account'" class="text-center py-8">
        <i class="pi pi-spin pi-spinner text-4xl text-blue-500 mb-4"></i>
        <h3 class="text-lg font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-2">
          Checking Smart Account
        </h3>
        <p class="text-sm text-gray-600 dark:text-gray-400">
          Verifying your smart account status...
        </p>
      </div>

      <!-- Loading: Cost Estimation -->
      <div v-else-if="currentStep === 'cost-estimation'" class="text-center py-8">
        <i class="pi pi-spin pi-spinner text-4xl text-blue-500 mb-4"></i>
        <h3 class="text-lg font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-2">
          Estimating Costs
        </h3>
        <p class="text-sm text-gray-600 dark:text-gray-400">
          Calculating gas costs...
        </p>
        <p v-if="signingMessage" class="text-sm text-blue-600 dark:text-blue-400 mt-4 font-medium">
          {{ signingMessage }}
        </p>
      </div>

      <!-- Smart Account Setup -->
      <div v-else-if="currentStep === 'smart-account-setup'" class="space-y-6">
        <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
          <div class="flex items-start gap-3">
            <i class="pi pi-info-circle text-blue-500 text-lg flex-shrink-0 mt-0.5"></i>
            <div>
              <h4 class="text-sm font-semibold text-blue-900 dark:text-blue-300 mb-1">
                {{ !smartAccount?.exists ? 'Create Smart Account' : 'Fund Smart Account' }}
              </h4>
              <p class="text-sm text-blue-700 dark:text-blue-400">
                <template v-if="!smartAccount?.exists">
                  To use gasless payments, you need to create and fund a smart account. This one-time setup allows you
                  to pay for gas fees using ANT tokens instead of ETH. You'll be asked to sign a permit in your wallet
                  to authorize the transfer.
                </template>
                <template v-else>
                  Your smart account needs additional funding to cover the payment and gas costs. You'll be asked to
                  sign a permit in your wallet to authorize the transfer.
                </template>
              </p>
            </div>
          </div>
        </div>

        <!-- Cost Breakdown - Only show if smart account exists (we can estimate gas) -->
        <div v-if="smartAccount?.exists" class="bg-gray-50 dark:bg-autonomi-blue-600 rounded-lg p-4">
          <h4 class="text-sm font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-3">
            Cost Breakdown
          </h4>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Storage Payment:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ formattedANT(costEstimate?.totalPaymentAmount || 0n) }} ANT
              </span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Gas for Payments ({{
                  costEstimate?.paymentBatchCount || 0
                }} batch(es)):</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ formattedANT(costEstimate?.estimatedPaymentGasCost || 0n) }} ANT
              </span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Gas for Funding:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ formattedANT(costEstimate?.estimatedFundingGasCost || 0n) }} ANT
              </span>
            </div>
            <div class="border-t border-gray-200 dark:border-gray-600 pt-2 mt-2">
              <div class="flex justify-between text-sm">
                <span class="text-gray-600 dark:text-gray-400">Total Cost:</span>
                <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                  {{ formattedANT((costEstimate?.totalPaymentAmount || 0n) + (costEstimate?.totalGasCost || 0n)) }} ANT
                </span>
              </div>
              <div class="flex justify-between text-sm mt-1">
                <span class="text-gray-600 dark:text-gray-400">Current Balance:</span>
                <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                  - {{ formattedANT(costEstimate?.currentSmartAccountBalance || 0n) }} ANT
                </span>
              </div>
              <div class="border-t border-gray-200 dark:border-gray-600 pt-2 mt-2">
                <div class="flex justify-between text-sm font-semibold">
                  <span class="text-gray-900 dark:text-autonomi-text-primary-dark">Additional Funding Needed:</span>
                  <span class="text-gray-900 dark:text-autonomi-text-primary-dark">
                    {{ formattedANT(costEstimate?.fundingAmount || 0n) }} ANT
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Simple explanation for first-time setup (no smart account yet) -->
        <div v-else class="bg-gray-50 dark:bg-autonomi-blue-600 rounded-lg p-4">
          <h4 class="text-sm font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-3">
            Initial Funding
          </h4>
          <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
            Choose how much ANT to fund your smart account with. This will cover:
          </p>
          <ul class="text-sm text-gray-600 dark:text-gray-400 space-y-1 ml-4 list-disc">
            <li>Storage payment costs</li>
            <li>Gas fees (paid in ANT instead of ETH)</li>
            <li>Optional: Extra funds for future transactions</li>
          </ul>
          <p class="text-xs text-gray-500 dark:text-gray-500 mt-3">
            Recommendation: Fund your account with enough ANT to cover future transactions. While you can add more funds
            later, doing so now will help reduce gas costs.
          </p>
        </div>

        <!-- Funding Amount Input -->
        <div>
          <label for="funding-amount"
                 class="block text-sm font-medium text-gray-900 dark:text-autonomi-text-primary-dark mb-2">
            Funding Amount (ANT)
          </label>
          <InputText
              id="funding-amount"
              v-model="userFundingAmount"
              type="text"
              placeholder="Enter amount (e.g., 1.5)"
              class="w-full"
              :class="{
              'p-invalid': insufficientBalance || belowMinimum
            }"
          />
          <p v-if="insufficientBalance" class="text-xs text-red-500 mt-1">
            Insufficient ANT balance. You have {{ antBalance }} ANT available.
          </p>
          <p v-else-if="belowMinimum" class="text-xs text-red-500 mt-1">
            Amount must be at least {{ formattedANT(costEstimate?.fundingAmount || 0n) }} ANT
          </p>
          <p v-else-if="!smartAccount?.exists" class="text-xs text-gray-600 dark:text-gray-400 mt-1">
            Enter the amount of ANT to fund your smart account. Available: {{ antBalance }} ANT
          </p>
          <p v-else class="text-xs text-gray-600 dark:text-gray-400 mt-1">
            You can add extra funds for future transactions. Available: {{ antBalance }} ANT
          </p>
        </div>

        <!-- Smart Account Info -->
        <div class="space-y-1">
          <div class="text-xs text-gray-600 dark:text-gray-400">
            <span class="font-medium">Smart Account:</span>
            <code class="ml-1 text-gray-900 dark:text-autonomi-text-primary-dark">{{ smartAccount?.address }}</code>
          </div>
          <div class="text-xs text-gray-600 dark:text-gray-400">
            <span class="font-medium">Current Balance:</span>
            <span class="ml-1 text-gray-900 dark:text-autonomi-text-primary-dark">
              {{ formattedANT(smartAccount?.antBalance || 0n) }} ANT
            </span>
          </div>
        </div>
      </div>

      <!-- Ready to Execute -->
      <div v-else-if="currentStep === 'executing-payments'" class="space-y-6">
        <div class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4">
          <div class="flex items-start gap-3">
            <i class="pi pi-check-circle text-green-500 text-lg flex-shrink-0 mt-0.5"></i>
            <div>
              <h4 class="text-sm font-semibold text-green-900 dark:text-green-300 mb-1">
                Ready for Gasless Payment
              </h4>
              <p class="text-sm text-green-700 dark:text-green-400">
                Your smart account has sufficient funds. When you proceed, you'll be asked to sign the payment
                transaction in your wallet.
              </p>
            </div>
          </div>
        </div>

        <!-- Cost Summary -->
        <div class="bg-gray-50 dark:bg-autonomi-blue-600 rounded-lg p-4">
          <h4 class="text-sm font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-3">
            Transaction Summary
          </h4>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Storage Payment:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ formattedANT(costEstimate?.totalPaymentAmount || 0n) }} ANT
              </span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Max Estimated Gas Cost:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                ~{{ formattedANT(costEstimate?.estimatedPaymentGasCost || 0n) }} ANT
              </span>
            </div>
            <div class="border-t border-gray-200 dark:border-gray-600 pt-2 mt-2">
              <div class="flex justify-between text-sm font-semibold">
                <span class="text-gray-900 dark:text-autonomi-text-primary-dark">Total:</span>
                <span class="text-gray-900 dark:text-autonomi-text-primary-dark">
                  {{ formattedANT(totalAntNeeded) }} ANT
                </span>
              </div>
            </div>
          </div>
        </div>

        <div class="text-xs text-gray-600 dark:text-gray-400">
          <span class="font-medium">Smart Account Balance:</span>
          <span class="ml-1 text-gray-900 dark:text-autonomi-text-primary-dark">
            {{ formattedANT(costEstimate?.currentSmartAccountBalance || 0n) }} ANT
          </span>
        </div>
      </div>

      <!-- Funding Smart Account -->
      <div v-else-if="currentStep === 'funding-smart-account'" class="text-center py-8">
        <i class="pi pi-spin pi-spinner text-4xl text-blue-500 mb-4"></i>
        <h3 class="text-lg font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-2">
          {{ !smartAccount?.exists ? 'Creating Smart Account' : 'Funding Smart Account' }}
        </h3>
        <p class="text-sm text-gray-600 dark:text-gray-400">
          Transferring {{ userFundingAmount }} ANT to your smart account...
        </p>
        <p v-if="signingMessage" class="text-sm text-blue-600 dark:text-blue-400 mt-4 font-medium">
          {{ signingMessage }}
        </p>
        <p v-else class="text-xs text-gray-500 dark:text-gray-500 mt-2">
          Please confirm the transaction in your wallet
        </p>
      </div>

      <!-- Error State -->
      <div v-else-if="currentStep === 'error'" class="space-y-4">
        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <div class="flex items-start gap-3">
            <i class="pi pi-exclamation-triangle text-red-500 text-lg flex-shrink-0 mt-0.5"></i>
            <div>
              <h4 class="text-sm font-semibold text-red-900 dark:text-red-300 mb-1">
                Setup Error
              </h4>
              <p class="text-sm text-red-700 dark:text-red-400">
                {{ error }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end items-center gap-3 mt-5">
        <Button
            label="Cancel"
            severity="secondary"
            text
            @click="handleCancel"
        />
        <Button
            v-if="canProceed"
            :label="!smartAccount?.exists && currentStep === 'smart-account-setup' ? 'Create Smart Account' : currentStep === 'smart-account-setup' ? 'Fund Smart Account' : 'Proceed with Payment'"
            :icon="isProcessing ? 'pi pi-spinner pi-spin' : undefined"
            severity="primary"
            @click="handleProceed"
            :disabled="!canProceed || isProcessing"
        />
      </div>
    </template>
  </Dialog>
</template>
