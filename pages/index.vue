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

const dummyFolders = ref([
  {
    name: "Folder 1",
    files: 10,
    size: "10MB",
  },
  {
    name: "Folder 2",
    files: 20,
    size: "20MB",
  },
  {
    name: "Folder 3",
    files: 30,
    size: "30MB",
  },
  {
    name: "Folder 4",
    files: 40,
    size: "40MB",
  },
  {
    name: "Folder 5",
    files: 50,
    size: "50MB",
  },
]);

const dummyFiles = ref([
  {
    name: "File 1.pdf",
    lastModified: "2021-10-10",
    owner: "John Doe",
    location: "Folder 1",
  },
  {
    name: "File 2.pdf",
    lastModified: "2021-10-11",
    owner: "Jane Doe",
    location: "Folder 2",
  },
  {
    name: "File 3.pdf",
    lastModified: "2021-10-12",
    owner: "John Doe",
    location: "Folder 3",
  },
  {
    name: "File 4.pdf",
    lastModified: "2021-10-13",
    owner: "Jane Doe",
    location: "Folder 4",
  },
  {
    name: "File 5.pdf",
    lastModified: "2021-10-14",
    owner: "John Doe",
    location: "Folder 5",
  },
]);

const handleGetStarted = () => {
  emit("open-login");
};
</script>

<template>
  <div>
    <div class="px-[66px] lg:px-[110px] pt-[70px]" v-if="!wallet.connected">
      <h1
        class="text-4xl font-semibold leading-[54px] text-autonomi-header-text"
      >
        Welcome to Autonomi
      </h1>
      <p class="mt-4 text-autonomi-text-primary">
        Welcome to the Autonomi vault.

        This is a secure private data store linked to your wallet, accessible from anywhere.

        You will need to link a wallet in order to pay for the upload and gas fees.

        Once you have linked your wallet, you will be able to see any files associated with it. You can also upload new files from here.

        To get started, press the button below.
      </p>

      <div v-if="!wallet.connected">
        <CommonButton
          variant="secondary"
          size="medium"
          @click="handleGetStarted"
          class="mt-10"
        >
          Get Started
        </CommonButton>
      </div>
    </div>
    <FileViewer v-else />
  </div>
</template>
