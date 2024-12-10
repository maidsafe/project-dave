// import { useAccount, useConnect, useDisconnect } from "@wagmi/vue";
// import { injected } from "@wagmi/connectors";
import type { Wallet } from "~/types/wallet";

export const useWalletStore = defineStore("wallet", () => {
  // State
  const wallet = ref<Wallet>({
    balance: 0,
    connected: false,
  });

  const pendingConnectWallet = ref(false);
  const pendingDisconnectWallet = ref(false);
  const openConnectWallet = ref(false);
  const openDisconnectWallet = ref(false);
  const callbackConnectWallet = ref<Function | null>(null);
  const callbackDisconnectWallet = ref<Function | null>(null);

  // const { connect } = useConnect();
  // const { disconnect } = useDisconnect();
  // const { address, isConnected } = useAccount();

  const connect: any = {};
  const disconnect: any = {};
  const address: any = ref({});
  const isConnected = ref(false);

  // Actions
  const tryConnectWallet = async () => {
    if (isConnected.value && address.value) {
      wallet.value.connected = true;
    }
  };

  const connectWallet = async () => {
    try {
      pendingConnectWallet.value = true;

      await connect({ connector: injected() });

      wallet.value.connected = true;

      // Trigger callback if it exists
      if (callbackConnectWallet.value) {
        callbackConnectWallet.value();
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

      wallet.value.connected = false;

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

  // Return
  return {
    // State
    pendingConnectWallet,
    pendingDisconnectWallet,
    openConnectWallet,
    openDisconnectWallet,
    wallet,
    address,
    isConnected,
    // Actions
    tryConnectWallet,
    connectWallet,
    disconnectWallet,
    hideConnectWallet,
    showConnectWallet,
    hideDisconnectWallet,
    showDisconnectWallet,
  };
});
