<script lang="ts" setup>
const classesLinks = `w-full h-[64px] text-lg flex items-center justify-start text-autonomi-text-primary hover:text-autonomi-text-secondary gap-3 transition-all duration-300 cursor-pointer`;
// const openLogin = ref(false);
// import { useWalletStore } from "~/stores/wallet";

// State
// const walletStore = useWalletStore();
// const autonomi = useAutonomiStore();
// const { openConnectWallet, openDisconnectWallet, wallet } =
// storeToRefs(walletStore);

const isFadeOut = ref(false);
const removeSplashScreen = ref(false);

const walletStore = ref<any>({}); // TODO: Remove me

// Computed

// Methods
const handleClickUpload = () => {
  try {
    return console.log(">>> TODO: handleClickUpload");
    if (wallet.value.connected) {
      navigateTo("/upload");
    } else {
      walletStore.showConnectWallet(() => {
        navigateTo("/upload");
      });
    }
  } catch (error) {
    // TODO: Handle error
    console.error(">>> Error: handleClickUpload");
  }
};

onMounted(async () => {
  // await autonomi.initWasm();

  setTimeout(() => {
    isFadeOut.value = true;

    // Remove splash screen
    setTimeout(() => {
      removeSplashScreen.value = true;
    }, 2000);
  }, 2000);
});
</script>

<template>
  <div class="min-h-screen flex flex-col bg-autonomi-gray-50 relative">
    <div
      v-if="!removeSplashScreen"
      class="absolute w-full h-full bg-white top-0 left-0 z-50 transition-all duration-1000"
      :class="{
        'opacity-100': !isFadeOut,
        'opacity-0': isFadeOut,
      }"
    >
      <div class="flex items-center justify-center h-full">
        <IconLogo />
      </div>
    </div>
    <NuxtLayout>
      <div class="sticky top-0 z-20">
        <Header />
      </div>
      <div class="flex flex-1">
        <!-- SideBar -->
        <div
          class="pb-4 w-[290px] transition-all duration-300 hidden lg:flex flex-col rounded-tr-2xl bg-white overflow-hidden items-center pt-[35px] shrink-0"
        >
          <div class="mb-11">
            <CommonButton
              variant="primary"
              size="large"
              @click="handleClickUpload"
              class="flex"
            >
              <i class="pi pi-plus-circle" /><span aria-label="Upload">
                Upload</span
              >
            </CommonButton>
          </div>

          <div class="flex flex-col justify-start">
            <NuxtLink :class="`${classesLinks}`" to="/">
              <IconFiles class="w-6 h-6" />
              Home
            </NuxtLink>

            <!-- <NuxtLink :class="`${classesLinks}`" to="/nodes">
              <div
                class="w-6 h-6 bg-autonomi-red-300 text-white flex items-center justify-center rounded-full"
              >
                <i class="pi pi-server text-xs" />
              </div>
              Nodes
            </NuxtLink>

            <NuxtLink :class="`${classesLinks}`" to="/wallet">
              <IconWallet class="w-6 h-6" />
              Wallet
            </NuxtLink>

            <NuxtLink :class="`${classesLinks}`" to="/settings">
              <IconSettings class="w-6 h-6" />
              Settings
            </NuxtLink> -->
          </div>
        </div>

        <div class="flex-1">
          <NuxtPage
            @open-login="walletStore.showConnectWallet"
            @close-login="walletStore.hideConnectWallet"
          />
          <Toast position="bottom-right" />
        </div>

        <!-- <DialogConnectWallet
          :visible="openConnectWallet"
          @close-login="walletStore.hideConnectWallet"
        /> -->

        <!-- <DialogDisconnectWallet
          :visible="openDisconnectWallet"
          @close-disconnect-wallet="walletStore.hideDisconnectWallet"
        /> -->
      </div>
    </NuxtLayout>
  </div>
</template>
