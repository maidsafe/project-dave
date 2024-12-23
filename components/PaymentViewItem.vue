<script lang="ts" setup>
import { usePaymentStore, ProcessingState } from "~/stores/payments";

const emit = defineEmits(["select-payment"]);
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
  <div
    class="flex flex-col gap-1 text-xs p-4 border border-gray-200 rounded-lg cursor-pointer hover:bg-gray-50 transition-all duration-300"
    @click="emit('select-payment', payment)"
  >
    <div class="flex justify-between items-center">
      <div class="flex flex-col gap-1">
        <div class="font-semibold text-sm">
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
      </div>
      <div>
        <i
          v-if="payment.processing === ProcessingState.COMPLETED"
          class="pi pi-check-circle"
        />
        <i
          v-else-if="payment.processing === ProcessingState.CANCELLED"
          class="pi pi-times text-red-600"
        />
        <i v-else class="pi pi-spin pi-spinner-dotted" />
      </div>
    </div>
  </div>
</template>
