<script lang="ts" setup>
// import { useToast } from "primevue/usetoast";
// import { useWalletStore } from "~/stores/wallet";
// import { storeToRefs } from "pinia";
// Login
const props = defineProps<{
  visible: boolean;
  notifyType: 'info' | 'warning';
  title: string;
  details: string;
}>();

const emit = defineEmits(["close-notify"]);
// const toast = useToast();
// const walletStore = useWalletStore();
// const { pendingDisconnectWallet, wallet } = storeToRefs(walletStore);

const { visible } = props;

// const handleCancelDisconnect = () => {
//   emit("close-disconnect-wallet");
// };
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
        class="flex flex-col px-8 py-8 gap-6 rounded-2xl bg-autonomi-blue-600 gap-y-1 "
      >
        <IconLogo :dark="true" />

        <div class="mt-6 text-white font-semibold flex items-center gap-2">
            <i class="pi pi-spin pi-spinner text-autonomi-red-300" />{{ title }}
        </div>
        <div
          class="flex items-center justify-center gap-4 text-autonomi-text-primary mt-2"
        >
          <span>{{  details  }}</span>
        </div>
      </div>
    </template>
  </Dialog>
</template>
