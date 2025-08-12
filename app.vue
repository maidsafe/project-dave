<script lang="ts" setup>
import {createAppKit} from '@reown/appkit/vue';
import {networks, projectId, wagmiAdapter} from '~/config';
import {useFileStore} from '~/stores/files';
import {storeToRefs} from 'pinia';
import {updater} from './lib/updater';
import {reconnect} from '@wagmi/core';

createAppKit({
  adapters: [wagmiAdapter],
  networks,
  metadata: {
    name: 'Autonomi',
    description: 'Autonomi',
    url: 'https://reown.com/appkit',
    icons: ['https://avatars.githubusercontent.com/u/179229932?s=200&v=4'],
  },
  projectId,
  features: {
    socials: false,
    email: false,
  },
});

const classesLinks = `w-full h-[64px] text-lg flex items-center justify-start text-autonomi-text-primary hover:text-autonomi-text-secondary gap-3 transition-all duration-300 cursor-pointer dark:hover:text-white`;

// State
const fileStore = useFileStore();
const walletStore = useWalletStore();
const {pendingFilesSignature} = storeToRefs(fileStore);

const {openConnectWallet, openDisconnectWallet, wallet} =
    storeToRefs(walletStore);
const notifyType = ref<'info' | 'warning'>('info');
const notifyTitle = ref('');
const notifyDetails = ref('');
const notifyCancelEnabled = ref(false);
const showNotification = ref(false);
const isFadeOut = ref(false);
const removeSplashScreen = ref(false);

// Methods

const handleShowNotification = (payload: any) => {
  console.log('>>> Notification payload:', payload);

  notifyType.value = payload.notifyType || 'info';
  notifyTitle.value = payload.title || '';
  notifyDetails.value = payload.details;
  notifyCancelEnabled.value = payload.enabledCancel || false;

  showNotification.value = true;
};

const handleHideNotification = () => {
  showNotification.value = false;
  pendingFilesSignature.value = false;
  notifyCancelEnabled.value = false;
};

watchEffect(() => {
  if (pendingFilesSignature.value) {
    handleShowNotification({
      notifyType: 'info',
      title: 'Sign file request',
      details:
          'To view your files please sign the file request in your mobile wallet.',
      enabledCancel: true,
    });
  } else {
    handleHideNotification();
  }
});

onMounted(async () => {
  // Attempt to reconnect wallet on app startup after a short delay
  // to ensure AppKit is fully initialized
  setTimeout(async () => {
    try {
      await reconnect(wagmiAdapter.wagmiConfig);
      console.log('Wallet reconnection attempted');
    } catch (error) {
      console.error('Failed to reconnect wallet:', error);
    }
  }, 100);

  setTimeout(() => {
    isFadeOut.value = true;

    // Remove splash screen
    setTimeout(() => {
      removeSplashScreen.value = true;
    }, 2000);
  }, 2000);
  updater();
});
</script>

<template>
  <div class="min-h-screen flex flex-col bg-autonomi-gray-50 dark:bg-autonomi-blue-600 relative">
    <div
        v-if="!removeSplashScreen"
        class="absolute w-full h-full bg-white top-0 left-0 z-50 transition-all duration-1000"
        :class="{
        'opacity-100': !isFadeOut,
        'opacity-0': isFadeOut,
      }"
    >
      <div class="flex items-center justify-center h-full">
        <IconLogo/>
      </div>
    </div>
    <NuxtLayout>
      <div class="sticky top-0 z-20">
        <Header/>
      </div>
      <div class="flex flex-1">
        <!-- SideBar -->
        <div
            class="pb-4 w-[290px] transition-all duration-300 hidden lg:flex flex-col rounded-tr-2xl bg-white dark:bg-white/10 overflow-hidden items-center pt-[35px] shrink-0"
        >
          <div class="flex flex-col justify-start">
            <NuxtLink :class="`${classesLinks}`" to="/">
              <IconFiles class="w-6 h-6"/>
              Vault
            </NuxtLink>

            <NuxtLink :class="`${classesLinks}`" to="/settings">
              <IconSettings class="w-6 h-6"/>
              About us
            </NuxtLink>
          </div>
        </div>

        <div class="flex-1">
          <NuxtPage
              @open-login="walletStore.showConnectWallet"
              @close-login="walletStore.hideConnectWallet"
              @show-notify="handleShowNotification"
              @hide-notify="handleHideNotification"
          />
          <Toast position="bottom-right"/>
        </div>

        <DialogConnectWallet
            :visible="openConnectWallet"
            @close-login="walletStore.hideConnectWallet"
        />

        <DialogDisconnectWallet
            :visible="openDisconnectWallet"
            @close-disconnect-wallet="walletStore.hideDisconnectWallet"
        />

        <DialogNotification
            :visible="showNotification"
            :notify-type="notifyType"
            :title="notifyTitle"
            :details="notifyDetails"
            :can-cancel="notifyCancelEnabled"
            @close-notify="handleHideNotification"
        />

      </div>
    </NuxtLayout>
  </div>
</template>
