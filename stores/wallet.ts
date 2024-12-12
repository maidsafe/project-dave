import { createAppKit } from "@reown/appkit/vue";
import { arbitrum, mainnet, type AppKitNetwork } from "@reown/appkit/networks";
import { WagmiAdapter } from "@reown/appkit-adapter-wagmi";
import { useAppKitAccount, useAppKit, useDisconnect } from "@reown/appkit/vue";

const projectId = "c57e0bb001a4dc96b54b9ced656a3cb8";

const metadata = {
  name: "maidsafe_ike_test",
  description: "AppKit Example",
  url: "https://reown.com/appkit", // origin must match your domain & subdomain
  icons: ["https://assets.reown.com/reown-profile-pic.png"],
};

const networks: [AppKitNetwork, ...AppKitNetwork[]] = [mainnet];

const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId,
});

const modal = createAppKit({
  adapters: [wagmiAdapter],
  networks,
  projectId,
  metadata,
  features: {},
});

export const useWalletStore = defineStore("wallet", () => {
  // State
  const pendingConnectWallet = ref(false);
  const pendingDisconnectWallet = ref(false);
  const openConnectWallet = ref(false);
  const openDisconnectWallet = ref(false);
  const callbackConnectWallet = ref<Function | null>(null);
  const callbackDisconnectWallet = ref<Function | null>(null);

  const wallet = ref(useAppKitAccount());
  const { open, close } = useAppKit();
  const { disconnect } = useDisconnect();

  // Actions
  const tryConnectWallet = async () => {
    console.log(">>> RUNNING tryConnectWallet, remove or update me");
  };

  const connectWallet = async () => {
    try {
      pendingConnectWallet.value = true;

      const connectResponse = await open();

      console.log(">>> Connect response: ", connectResponse);

      // Trigger callback if it exists
      // TODO: Update this callback functionality, do we even need it?
      if (callbackConnectWallet.value) {
        callbackConnectWallet.value();
      }

      // TODO: Fix how to handle a response using appKit
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
