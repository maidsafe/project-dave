<script lang="ts" setup>
import {usePaymentStore} from "~/stores/payments";
import {storeToRefs} from "pinia";

interface UploadStep {
  key: string;
  label: string;
  status: 'pending' | 'processing' | 'completed' | 'error';
  message?: string;
  progress?: number;
}

interface QuoteData {
  totalFiles: number;
  totalSize: string;
  totalCostFormatted?: string;
  pricePerMB?: string;
  paymentRequired?: boolean;
  paymentOrderId?: string;
  totalCostNano?: string;
  costPerFileNano?: string;
  rawQuoteData?: any;
}

const props = defineProps<{
  visible: boolean;
  currentStep?: string;
  steps?: UploadStep[];
  quoteData?: QuoteData;
  error?: string;
}>();

const emit = defineEmits(["close-modal", "cancel-upload", "show-notify", "hide-notify", "pay-upload"]);

const paymentStore = usePaymentStore();
const {
  currentPayment,
  pendingPayments,
  signPaymentPending,
  sortedPendingPayments,
} = storeToRefs(paymentStore);

const handleSelectPayment = (payment: any) => {
  console.log(">>> handleSelectPayment", payment);

  // Reset payment timer
  paymentStore.resetExpirationTime(payment.order.id);

  // Set current payment to payment
  paymentStore.setCurrentPayment(payment);

  // Set view to payment
  paymentStore.setPaymentView("payment");
};

const handlePayment = () => {
  paymentStore.pay(currentPayment.value.order);
};

const handleCancel = () => {
  if (currentPayment.value) {
    paymentStore.cancel(currentPayment.value.order.id);
  }
  emit("cancel-upload");
};

const canClose = computed(() => {
  return !props.steps?.some(step => step.status === 'processing');
});

const showQuoteData = computed(() => {
  // Show quote data only when we have the payment request
  return props.quoteData && props.currentStep === 'payment-request' && props.quoteData.totalCostFormatted;
});

const showPendingPayments = computed(() => {
  return sortedPendingPayments.value && sortedPendingPayments.value.length > 0;
});

const hasActivePayment = computed(() => {
  return currentPayment.value && props.currentStep === 'payment-request';
});

const isPaymentProcessing = computed(() => {
  return props.steps?.some(step => step.key === 'payment-request' && step.status === 'processing' && step.message?.includes('wallet authorization'));
});

const totalPaymentAmount = computed(() => {
  if (currentPayment.value) {
    return paymentStore.calculateTotalAmount(currentPayment.value.order.payments);
  }
  return props.quoteData?.totalCostNano || '0';
});

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

// Timer for payment expiration
const remainingTime = ref("00:00:00");
let interval: any;

const startCountdown = () => {
  if (currentPayment.value) {
    remainingTime.value = paymentStore.calculateRemainingTime(currentPayment.value.expires);

    interval = setInterval(() => {
      if (currentPayment.value) {
        const time = paymentStore.calculateRemainingTime(currentPayment.value.expires);
        remainingTime.value = time;

        if (time === "00:00:00") {
          clearInterval(interval);
        }
      }
    }, 1000);
  }
};

watchEffect(() => {
  if (currentPayment.value && props.currentStep === 'payment-request') {
    startCountdown();
  }
});

onUnmounted(() => {
  if (interval) {
    clearInterval(interval);
  }
});

watchEffect(() => {
  if (signPaymentPending.value) {
    emit("show-notify", {
      title: "Payment request",
      details: "Please sign the payment request in your mobile wallet app.",
      notifyType: "info",
    });
  } else {
    emit("hide-notify");
  }
});
</script>

<template>
  <Dialog
      :visible="props.visible"
      modal
      header="Upload Progress"
      :style="{width: '32rem'}"
      position="center"
      :closable="canClose"
      :draggable="false"
      pt:root:class="!border-0"
      pt:mask:class="backdrop-blur-sm"
      @hide="emit('close-modal')"
  >
    <template #header>
      <div class="flex items-center justify-between w-full">
        <div class="flex items-center gap-3">
          <img src="~/assets/img/autonomi-logo-text-white.svg" alt="Autonomi" class="h-6 filter invert dark:filter-none" />
        </div>
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 bg-blue-500 rounded-full flex items-center justify-center">
            <i class="pi pi-upload text-white"/>
          </div>
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-autonomi-text-primary-dark">
              Upload Progress
            </h3>
            <p class="text-sm text-gray-500 dark:text-gray-400">
              Getting quote for your files
            </p>
          </div>
        </div>
      </div>
    </template>

    <div class="space-y-6">
      <!-- Progress Steps -->
      <div v-if="steps && steps.length > 0" class="space-y-3">
        <div v-for="step in steps" :key="step.key"
             class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 dark:border-autonomi-gray-700">
          <!-- Status Icon -->
          <div class="flex-shrink-0">
            <div v-if="step.status === 'completed'"
                 class="w-8 h-8 bg-green-500 rounded-full flex items-center justify-center">
              <i class="pi pi-check text-white text-sm"/>
            </div>
            <div v-else-if="step.status === 'processing'"
                 class="w-8 h-8 bg-blue-500 rounded-full flex items-center justify-center">
              <i class="pi pi-spinner pi-spin text-white text-sm"/>
            </div>
            <div v-else-if="step.status === 'error'"
                 class="w-8 h-8 bg-red-500 rounded-full flex items-center justify-center">
              <i class="pi pi-times text-white text-sm"/>
            </div>
            <div v-else class="w-8 h-8 bg-gray-300 dark:bg-gray-600 rounded-full flex items-center justify-center">
              <i class="pi pi-clock text-gray-500 dark:text-gray-400 text-sm"/>
            </div>
          </div>

          <!-- Step Content -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between">
              <h4 class="text-sm font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ step.label }}
              </h4>
              <span v-if="step.progress !== undefined" class="text-xs text-gray-500 dark:text-gray-400">
                {{ step.progress }}%
              </span>
            </div>
            <p v-if="step.message" class="text-sm text-gray-600 dark:text-gray-400 mt-1 truncate">
              {{ step.message }}
            </p>
            <!-- Progress bar for processing steps -->
            <div v-if="step.status === 'processing' && step.progress !== undefined" class="mt-2">
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                <div class="bg-blue-500 h-1.5 rounded-full transition-all duration-300"
                     :style="`width: ${step.progress}%`"></div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Error Display -->
      <div v-if="error" class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <div class="flex items-start gap-3">
          <i class="pi pi-exclamation-triangle text-red-500 text-lg flex-shrink-0 mt-0.5"/>
          <div class="flex-1">
            <h4 class="text-sm font-semibold text-red-900 dark:text-red-300 mb-1">
              Upload Error
            </h4>
            <p class="text-sm text-red-700 dark:text-red-400">
              {{ error }}
            </p>
          </div>
        </div>
      </div>

      <!-- Quote Data (when payment is requested) -->
      <div v-if="showQuoteData && !error" class="space-y-4">
        <!-- Upload Summary -->
        <div class="bg-gray-50 dark:bg-autonomi-gray-800 rounded-lg p-4">
          <h4 class="text-sm font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-3">
            Upload Summary
          </h4>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Files:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ quoteData?.totalFiles }}
              </span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Total Size:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ quoteData?.totalSize }}
              </span>
            </div>
            <div v-if="quoteData?.pricePerMB" class="flex justify-between text-sm">
              <span class="text-gray-600 dark:text-gray-400">Price per MB:</span>
              <span class="font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ quoteData?.pricePerMB }}
              </span>
            </div>
          </div>
        </div>

        <!-- Storage Cost / Payment Request -->
        <div class="bg-gray-50 dark:bg-autonomi-gray-800 rounded-lg p-4">
          <h4 class="text-sm font-semibold text-gray-900 dark:text-autonomi-text-primary-dark mb-3">
            Payment Request
          </h4>
          <div class="space-y-3">
            <div class="flex justify-between text-sm">
              <span class="font-semibold text-gray-900 dark:text-autonomi-text-primary-dark">Total Cost:</span>
              <span class="font-bold text-blue-600 dark:text-blue-400">
                {{ quoteData?.totalCostFormatted || '0 ATTO' }}
              </span>
            </div>
          </div>
        </div>

      </div>


    </div>

    <template #footer>
      <div class="flex justify-end items-center gap-3 py-3">
        <Button
            label="Cancel"
            severity="secondary"
            text
            @click="handleCancel"
        />
        <Button
            v-if="currentStep === 'payment-request' && quoteData?.paymentRequired !== false"
            :label="isPaymentProcessing ? 'Processing...' : 'Pay & Upload'"
            :icon="isPaymentProcessing ? 'pi pi-spinner pi-spin' : 'pi pi-wallet'"
            severity="primary"
            :disabled="isPaymentProcessing"
            @click="$emit('pay-upload')"
        />
      </div>
    </template>
  </Dialog>
</template>