import { useWalletStore } from "./wallet";
import { invoke } from "@tauri-apps/api/core";

export type PaymentOrder = {
  id: number;
  payments: [[string, string, string]];
};

export enum ProcessingState {
  PENDING,
  PROCESSING,
  COMPLETED,
  CANCELLED,
}

export type PendingPayment = {
  order: PaymentOrder;
  expires: number;
  processing: ProcessingState;
};

type PaymentViews = "list" | "payment";

export const usePaymentStore = defineStore("payments", () => {
  const walletStore = useWalletStore();

  // State
  const IDLE_PAYMENT_EXPIRATION_TIME_SECS = ref<number>(600);
  const currentPayment = ref<any>(null);
  const pendingPayments: Ref<Map<number, PendingPayment>> = ref(new Map());
  const showPayments = ref(false);
  const paymentView = ref<PaymentViews>("list");
  const showPaymentDrawer = ref(false);
  const signPaymentPending = ref(false);

  const pendingPaymentsCount = computed(
    () =>
      Array.from(pendingPayments.value.values()).filter(
        (payment) =>
          payment.processing === ProcessingState.PENDING ||
          payment.processing === ProcessingState.PROCESSING
      ).length
  );

  const sortedPendingPayments = computed(() => {
    return Array.from(pendingPayments.value.values())
      .filter((payment) => payment.expires > Date.now())
      .sort((a, b) => a.expires - b.expires);
  });

  // Methods
  const openPaymentDrawer = () => {
    setPaymentView("list");
    showPaymentDrawer.value = true;
  };

  const closePaymentDrawer = () => {
    setPaymentView("list");
    showPaymentDrawer.value = false;
  };

  const addPendingPayment = (orderId: number, payment: any) => {
    console.log(">>> ADDING PAYMENT");
    pendingPayments.value.set(orderId, payment);

    // Update current payment
    currentPayment.value = pendingPayments.value.get(orderId);

    openPaymentDrawer();
  };

  const resetExpirationTime = (orderId: number) => {
    const payment = pendingPayments.value.get(orderId);

    if (payment) {
      payment.expires =
        Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS.value; // Reset expiration to 120 seconds from now
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

  const setCurrentPayment = (payment: any) => {
    // TODO: Use correc ttype
    currentPayment.value = payment;
  };

  //   const calculateRemainingTime = (expires: number) =>
  //     Math.max(0, Math.floor((expires - Date.now()) / 1000));

  const calculateRemainingTime = (expires: number) => {
    const totalSeconds = Math.max(0, Math.floor((expires - Date.now()) / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(
      2,
      "0"
    )}:${String(seconds).padStart(2, "0")}`;
  };

  const calculateTotalAmount = (
    payments: [[string, string, string]]
  ): bigint => {
    return payments.reduce((total, payment) => {
      const amountHex = payment[2];
      const amount = BigInt(amountHex);
      return total + amount;
    }, BigInt(0));
  };

  const pay = async (order: PaymentOrder) => {
    let processingState = getProcessingState(order.id);

    if (
      processingState === ProcessingState.PROCESSING ||
      processingState === ProcessingState.COMPLETED
    ) {
      return;
    }

    try {
      console.log(">>> Attempting to pay for order", order);
      // Set payment pending - flag to show notification
      signPaymentPending.value = true;

      await invoke("send_payment_order_message", {
        id: order.id,
        message: "KeepAlive",
      });

      // Set sign pending false - flag to remove notification
      signPaymentPending.value = false;

      setProcessingState(order.id, ProcessingState.PROCESSING);
      resetExpirationTime(order.id);
      await walletStore.payForQuotes(order.payments);
      setProcessingState(order.id, ProcessingState.COMPLETED);
      await invoke("send_payment_order_message", {
        id: order.id,
        message: "Completed",
      });
      console.log(">>> Payment complete");
    } catch (err) {
      console.error(">>> Error paying for quote", err);

      // Set sign pending false - flag to remove notification
      signPaymentPending.value = false;

      setProcessingState(order.id, ProcessingState.CANCELLED);
      await invoke("send_payment_order_message", {
        id: order.id,
        message: "Cancelled",
      });
    }
  };

  const cancel = async (orderId: number) => {
    setProcessingState(orderId, ProcessingState.CANCELLED);
    await invoke("send_payment_order_message", {
      id: orderId,
      message: "Cancelled",
    });
  };

  const setPaymentView = (view: PaymentViews) => {
    paymentView.value = view;
  };

  // Return values
  return {
    currentPayment,
    IDLE_PAYMENT_EXPIRATION_TIME_SECS,
    pendingPayments,
    pendingPaymentsCount,
    signPaymentPending,
    showPaymentDrawer,
    sortedPendingPayments,
    paymentView,
    addPendingPayment,
    calculateRemainingTime,
    calculateTotalAmount,
    closePaymentDrawer,
    getProcessingState,
    openPaymentDrawer,
    resetExpirationTime,
    setCurrentPayment,
    setPaymentView,
    pay,
    cancel,
  };
});
