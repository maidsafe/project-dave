<script lang="ts" setup>
import { ref } from "vue";
import { useToast } from "primevue/usetoast";
import { useWalletStore } from "~/stores/wallet";
import { storeToRefs } from "pinia";

useHeadSafe({
  script: [{ id: "xss-script", innerHTML: 'alert("xss")' }],
  title: "Autonomi",
  meta: [{ "http-equiv": "refresh", content: "0;javascript:alert(1)" }],
});

// Emits
const emit = defineEmits(["open-login", "close-login"]);

// Composables
const toast = useToast();

// State
const walletStore = useWalletStore();
const { wallet } = storeToRefs(walletStore);

const handleGetStarted = () => {
  emit("open-login");
};
</script>

<template>
  <div class="min-h-screen bg-white dark:bg-gray-900 text-black dark:text-white">
    <div class="px-[66px] lg:px-[110px] pt-[70px]" v-if="!wallet.isConnected">
      <h1 class="text-4xl font-semibold leading-[54px] text-autonomi-header-text dark:text-white">
        Welcome to Autonomi
      </h1>
      <p class="mt-4 text-autonomi-text-primary dark:text-gray-300">
        Welcome to the Autonomi vault. This is a secure private data store
        linked to your wallet, accessible from anywhere. You will need to link a
        wallet in order to pay for the upload and gas fees. Once you have linked
        your wallet, you will be able to see any files associated with it. You
        can also upload new files from here. To get started, press the button
        below.
      </p>

      <div v-if="!wallet.isConnected">
        <CommonButton
          variant="secondary"
          size="medium"
          @click="handleGetStarted"
          class="mt-10"
        >
          Get Started
        </CommonButton>
      </div>

      <!-- <div class="p-10 bg-green-200">
        <div class="mt-10">Data: {{ wallet }}</div>
      </div> -->
    </div>
    <FileViewer v-else />
  </div>
</template>
