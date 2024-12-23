<script lang="ts" setup>
import { usePaymentStore } from "~/stores/payments";
import { storeToRefs } from "pinia";

const paymentStore = usePaymentStore();
const {
  currentPayment,
  showPaymentDrawer,
  paymentView,
  pendingPayments,
  pendingPaymentsCount,
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
  //   paymentView.value = "payment";
  paymentStore.setPaymentView("payment");
};

const handlePayment = () => {
  paymentStore.pay(currentPayment.value.order);
};
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
      <template v-if="paymentView === 'payment'">
        <!-- PAYMENT VIEW -->
        <PaymentViewPay
          :payment="currentPayment"
          @payment-pay="handlePayment"
          @payment-cancel="paymentStore.cancel(currentPayment.order.id)"
        />
      </template>
      <template v-else>
        <!-- DEFAULT LIST VIEW -->
        <div class="w-full">
          <div
            v-if="sortedPendingPayments?.length > 0"
            class="flex flex-col gap-4"
          >
            <PaymentViewItem
              v-for="payment in sortedPendingPayments"
              :key="payment.order.id"
              :payment="payment"
              @select-payment="handleSelectPayment"
            />
          </div>
          <div v-else>No current payment requests.</div>
        </div>
      </template>
    </div>
  </Drawer>
</template>
