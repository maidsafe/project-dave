<script lang="ts" setup>
import { useToast } from "primevue/usetoast";
import { useWalletStore } from "~/stores/wallet";
import { storeToRefs } from "pinia";
// Login
const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits(["close-login"]);
const toast = useToast();
const walletStore = useWalletStore();
const { pendingConnectWallet, wallet, callbackConnectWallet } = storeToRefs(walletStore);

const { visible } = props;

const handleCloseLogIn = () => {
  emit("close-login");
};

const handleLogIn = async () => {
  try {
    console.log(">>> Running login");

    const response = await walletStore.connectWallet();

    console.log(">>> Response", response);

    if (response.success) {
      if (callbackConnectWallet.value) {
        callbackConnectWallet.value();
      }
    } else {
      throw new Error("Failed to log in");
    }
  } catch (error) {
    toast.add({
      severity: "error",
      summary: "Error",
      detail: "TEST: Failed to log in",
      life: 3000,
    });
  } finally {
    emit("close-login");
  }
};
</script>

<template>
  <Dialog
    :visible="props.visible"
    pt:root:class="!border-0 !bg-transparent"
    pt:mask:class="backdrop-blur-sm"
    position="topright"
  >
    <template #container="{ closeCallback }">
      <div
        class="flex flex-col px-8 py-8 gap-6 rounded-2xl bg-autonomi-blue-600 dark:bg-black"
      >
        <IconLogo :dark="true" />

        <div
          v-if="pendingConnectWallet || wallet.connected"
          class="flex items-center justify-center gap-4 text-autonomi-text-primary"
        >
          <i class="pi pi-spin pi-spinner" />
          <span>Connecting...</span>
        </div>
        <div v-else class="flex items-center gap-4 flex-wrap justify-center">
          <CommonButton
            variant="tertiary"
            size="large"
            @click="handleCloseLogIn"
          >
            Cancel
          </CommonButton>
          <CommonButton variant="primary" size="large" @click="handleLogIn">
            <i class="pi pi-wallet" /> Connect Your Mobile Wallet
          </CommonButton>
        </div>
      </div>
    </template>
  </Dialog>
</template>
