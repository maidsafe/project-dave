<script lang="ts" setup>
import {listen} from "@tauri-apps/api/event";
import {invoke} from "@tauri-apps/api/core";
import {useWalletStore} from "~/stores/wallet";

const IDLE_PAYMENT_EXPIRATION_TIME_SECS = 600;

type PaymentOrder = {
  id: number;
  payments: [[string, string, string]];
};

enum ProcessingState {
  PENDING,
  PROCESSING,
  COMPLETED,
  CANCELLED,
}

type PendingPayment = {
  order: PaymentOrder;
  expires: number;
  processing: ProcessingState;
}

const walletStore = useWalletStore();

const pendingPayments: Ref<Map<number, PendingPayment>> = ref(new Map());
const visible = ref(false);

const pendingPaymentsCount = computed(() =>
    Array.from(pendingPayments.value.values())
        .filter(payment => payment.processing === ProcessingState.PENDING || payment.processing === ProcessingState.PROCESSING)
        .length
);

const sortedPendingPayments = computed(() => {
  return Array.from(pendingPayments.value.values())
      .filter(payment => payment.expires > Date.now())
      .sort((a, b) => a.expires - b.expires);
});

const resetExpirationTime = (orderId: number) => {
  const payment = pendingPayments.value.get(orderId);

  if (payment) {
    payment.expires = Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS; // Reset expiration to 120 seconds from now
    pendingPayments.value.set(orderId, payment);
  } else {
    console.error(`Order with ID ${orderId} not found in pending payments.`);
  }
};

const setProcessingState = (orderId: number, state: ProcessingState) => {
  const payment = pendingPayments.value.get(orderId);

  if (payment) {
    payment.processing = state;
    pendingPayments.value.set(orderId, payment);
  } else {
    console.error(`Order with ID ${orderId} not found in pending payments.`);
  }
};

const getProcessingState = (orderId: number): ProcessingState | undefined => {
  const payment = pendingPayments.value.get(orderId);
  return payment ? payment.processing : undefined;
};

const calculateRemainingTime = (expires: number) => Math.max(0, Math.floor((expires - Date.now()) / 1000));

const calculateTotalAmount = (payments: [[string, string, string]]): bigint => {
  return payments.reduce((total, payment) => {
    const amountHex = payment[2];
    const amount = BigInt(amountHex);
    return total + amount;
  }, BigInt(0));
};

const pay = async (order: PaymentOrder) => {
  let processingState = getProcessingState(order.id);

  if (processingState === ProcessingState.PROCESSING || processingState === ProcessingState.COMPLETED) {
    return;
  }

  try {
    await invoke("send_payment_order_message", {id: order.id, message: "KeepAlive"});
    setProcessingState(order.id, ProcessingState.PROCESSING);
    resetExpirationTime(order.id);
    await walletStore.payForQuotes(order.payments);
    setProcessingState(order.id, ProcessingState.COMPLETED);
    await invoke("send_payment_order_message", {id: order.id, message: "Completed"});
  } catch (err) {
    console.error(">>> Error paying for quote", err);
    setProcessingState(order.id, ProcessingState.CANCELLED);
    await invoke("send_payment_order_message", {id: order.id, message: "Cancelled"});
  }
}

const cancel = async (orderId: number) => {
  setProcessingState(orderId, ProcessingState.CANCELLED)
  await invoke("send_payment_order_message", {id: orderId, message: "Cancelled"});
}

listen<PaymentOrder>("payment-order", async (event: any) => {
  let order: PaymentOrder = JSON.parse(event.payload);

  if (!order) return;

  console.log(">>> Payment order received", order);

  pendingPayments.value.set(order.id, {
    order,
    expires: Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS,
    processing: ProcessingState.PENDING,
  });

  visible.value = true;
});
</script>

<template>
  <div class="card flex justify-center">
    <Button class="hidden lg:flex gap-1" type="button" icon="pi pi-receipt" label="Pay"
            :severity="pendingPaymentsCount > 0 ? 'info' : 'secondary'"
            :badge="pendingPaymentsCount.toString()"
            @click="visible = true"/>

    <!-- MOBILE -->
    <Button class="flex lg:hidden gap-1" type="button" icon="pi pi-receipt"
            :severity="pendingPaymentsCount > 0 ? 'info' : 'secondary'"
            :badge="pendingPaymentsCount.toString()"
            @click="visible = true"/>

    <Dialog v-model:visible="visible" modal dismissable-mask header="Payment requests" :style="{ width: '25rem' }">
      <template v-if="pendingPaymentsCount > 0">
        <span class="text-surface-500 dark:text-surface-400 block mb-8">Pay using your wallet app before the timer runs out.</span>
      </template>
      <template v-else>
        <span class="text-surface-500 dark:text-surface-400 block mb-8">No pending payments.</span>
      </template>

      <div class="payment-orders">
        <ul class="list-none p-0">
          <li v-for="(payment, index) in sortedPendingPayments" :key="payment.order.id" class="mb-4"
              :class="{ 'border-t border-surface-200 dark:border-surface-700': index !== 0 }">
            <div class="p-3 border-round border-1 surface-border flex justify-between items-center gap-4">
              <div class="flex flex-col">
                <p class="text-sm">Order ID: {{ payment.order.id }}</p>
                <p class="text-sm">Total amount: {{ calculateTotalAmount(payment.order.payments) }} ATTO</p>
                <template
                    v-if="getProcessingState(payment.order.id) === ProcessingState.PENDING || getProcessingState(payment.order.id) === ProcessingState.PROCESSING">
                  <p class="text-sm">Expires in: {{ calculateRemainingTime(payment.expires) }} seconds</p>
                </template>
              </div>

              <template v-if="getProcessingState(payment.order.id) === ProcessingState.PENDING">
                <Button label="Pay" icon="pi pi-wallet" class="flex gap-1" @click="pay(payment.order)"/>
                <Button label="Cancel" icon="pi pi-times" class="flex gap-1" @click="cancel(payment.order.id)"/>
                >
              </template>
              <template v-if="getProcessingState(payment.order.id) === ProcessingState.PROCESSING">
                <Button label="Pay" icon="pi pi-wallet" class="flex gap-1" disabled/>
              </template>
              <template v-if="getProcessingState(payment.order.id) === ProcessingState.COMPLETED">
                <i class="pi pi-check-circle" style="font-size: 1.5rem; color: green"/>
              </template>
              <template v-if="getProcessingState(payment.order.id) === ProcessingState.CANCELLED">
                <i class="pi pi-times-circle" style="font-size: 1.5rem; color: red"/>
              </template>
            </div>
          </li>
        </ul>
      </div>
    </Dialog>
  </div>
</template>
