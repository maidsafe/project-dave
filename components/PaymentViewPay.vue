<script lang="ts" setup>
import { usePaymentStore, ProcessingState } from "~/stores/payments";

const emit = defineEmits(["payment-pay", "payment-cancel"]);
const paymentStore = usePaymentStore();
const { payment } = defineProps<{
  payment: any;
}>();

const remainingTime = ref("00:00:00");
let interval: any;

// Start the countdown
const startCountdown = () => {
  remainingTime.value = paymentStore.calculateRemainingTime(payment.expires);

  interval = setInterval(() => {
    const time = paymentStore.calculateRemainingTime(payment.expires);
    remainingTime.value = time;

    if (time === "00:00:00") {
      clearInterval(interval);
    }
  }, 1000);
};

onMounted(() => {
  startCountdown();
});

onUnmounted(() => {
  if (interval) {
    clearInterval(interval);
  }
});
</script>

<template>
  <div class="flex flex-col gap-1 text-xs p-4 w-full">
    <div
      class="text-sm font-semibold"
      v-if="payment.processing !== ProcessingState.COMPLETED"
    >
      Pay using your wallet app before the timer runs out.
    </div>
    <div class="font-semibold text-sm mt-4">
      Payment ID: {{ payment.order.id }}
    </div>
    <div>
      Total amount:
      {{ paymentStore.calculateTotalAmount(payment.order.payments) }} ATTO
    </div>
    <div
      v-if="
        paymentStore.getProcessingState(payment.order.id) ===
        ProcessingState.PENDING
      "
    >
      Expires in: {{ remainingTime }}
    </div>
    <div class="mt-10 flex items-center justify-center gap-6">
      <template v-if="payment.processing === ProcessingState.COMPLETED">
        <div class="flex gap-2 items-center font-semibold text-green-600">
          <i class="pi pi-check-circle" />
          Payment Complete
        </div>
      </template>
      <template v-else-if="payment.processing === ProcessingState.CANCELLED">
        <div class="flex gap-2 items-center font-semibold text-red-600">
          <i class="pi pi-times text-red-600" />
          Payment Cancelled
        </div>
      </template>
      <template v-else>
        <CommonButton
          label="Pay"
          icon="pi pi-wallet"
          class="flex gap-1"
          variant="secondary"
          @click="emit('payment-pay', payment)"
        >
          <i class="pi pi-wallet" />
          Pay
        </CommonButton>
        <CommonButton
          label="Cancel"
          icon="pi pi-times"
          class="flex gap-1"
          variant="tertiary"
          @click="emit('payment-cancel', payment)"
        >
          <i class="pi pi-times" />
          Cancel
        </CommonButton>
      </template>
    </div>
  </div>
</template>
