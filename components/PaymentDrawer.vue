<script lang="ts" setup>
import { usePaymentStore } from "~/stores/payments";
import { storeToRefs } from "pinia";

const emit = defineEmits(["show-notify", "hide-notify"]);

const paymentStore = usePaymentStore();
const {
  currentPayment,
  showPaymentDrawer,
  pendingPayments,
  pendingPaymentsCount,
  signPaymentPending,
  sortedPendingPayments,
} = storeToRefs(paymentStore);
const handleSelectPayment = (payment: any) => {
  console.log(">>> handleSelectPayment", payment);
  // TODO: CHeck payment status, should we show payment or signing view or do nothing?

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

watchEffect(() => {
  if (signPaymentPending.value) {
    emit("show-notify", {
      title: "Payment request",
      details: "Please sign the payment request in your wallet app.",
      notifyType: "info",
    });
  } else {
    emit("hide-notify");
  }
});
</script>

<template>
  <!-- FORWARDERS DRAWER -->
  <Drawer
    v-model:visible="showPaymentDrawer"
    :header="`Payment requests ${
      pendingPaymentsCount > 0 ? `(${pendingPaymentsCount})` : ''
    }`"
    position="right"
    class="!h-auto !w-[380px] rounded-l-2xl"
  >
    <div
      class="border-t border-t-autonomi-text-primary/10 flex flex-col items-center py-7 w-[95%] mx-auto"
    >
      <div class="w-full">
        <div
          v-if="sortedPendingPayments?.length > 0"
          class="flex flex-col gap-4"
        >
          <PaymentViewItem
            v-for="payment in sortedPendingPayments"
            :key="payment.order.id"
            :payment="payment"
            @payment-pay="handlePayment"
            @payment-cancel="paymentStore.cancel(currentPayment.order.id)"
          />
        </div>
        <div v-else>No current payment requests.</div>
      </div>
    </div>
  </Drawer>
</template>
