<script lang="ts" setup>
import { createAppKit } from '@reown/appkit/vue';
import { networks, projectId, wagmiAdapter } from '~/config';
import { useFileStore } from '~/stores/files';
import { storeToRefs } from 'pinia';
import { updater } from './lib/updater';

// Initialize AppKit
createAppKit({
  adapters: [wagmiAdapter],
  networks,
  projectId,
  metadata: {
    name: 'Autonomi',
    description: 'Autonomi',
    url: 'https://reown.com/appkit',
    icons: ['https://avatars.githubusercontent.com/u/179229932?s=200&v=4'],
  },
  features: {
    email: false,
    socials: false,
  },
  includeWalletIds: [
    'c57ca95b47569778a828d19178114f4db188b89b763c899ba0be274e97267d96', // Metamask
    '4622a2b2d6af1c9844944291e5e7351a6aa24cd7b23099efac1b2fd875da31a0', // Trust wallet
    '971e689d0a5be527bac79629b4ee9b925e82208e5168b733496a09c0faed0709', // OKX Wallet
    '8a0ee50d1f22f6651afcae7eb4253e52a3310b90af5daef78a8c4929a9bb99d4', // Binance Wallet
    '38f5d18bd8522c244bdd70cb4a68e0e718865155811c043f052fb9f1c51de662', // Bitget Wallet
    '0b415a746fb9ee99cce155c2ceca0c6f6061b1dbca2d722b3ba16381d0562150', // SafePal
    'c03dfee351b6fcc421b4494ea33b9d4b92a984f87aa76d1663bb28705e95034a', // Uniswap Wallet
    '15c8b91ade1a4e58f3ce4e7a0dd7f42b47db0c8df7e0d84f63eb39bcb96c4e0f', // Bybit Wallet
    '20459438007b75f4f4acb98bf29aa3b800550309646d375da5fd4aac6c2a2c66', // Token Pocket
    '19177a98252e07ddfc9af2083ba8e07ef627cb6103467ffebb3f8f4205fd7927', // Ledger Live
    '1ae92b26df02f0abca6304df07debccd18262fdf5fe82daa81593582dac9a369', // Rainbow
    'f2436c67184f158d1beda5df53298ee84abfc367581e4505134b5bcf5f46697d', // Crypto.com
    'ecc4036f814562b41a5268adc86270fba1365471402006302e70169465b7ac18', // Zerion
    'ef333840daf915aafdc4a004525502d6d49d77bd9c65e0642dbaefb3c2893bef', // imToken
    'c286eebc742a537cd1d6818363e9dc53b21759a1e8e5d9b263d0c03ec7703576', // 1inch Wallet
    '18450873727504ae9315a084fa7624b5297d2fe5880f0982979c17345a138277', // Kraken Wallet
    '225affb176778569276e484e1b92637ad061b01e13a048b35a9d280c3b58970f', // Safe 
    '344d0e58b139eb1b6da0c29ea71d52a8eace8b57897c6098cb9b46012665c193', // Timeless X
    '541d5dcd4ede02f3afaf75bf8e3e4c4f1fb09edb5fa6c4377ebf31c2785d9adf', // Ronin Wallet
  ]
});

const classesLinks = `w-full h-[64px] text-lg flex items-center justify-start text-autonomi-text-primary hover:text-autonomi-text-secondary gap-3 transition-all duration-300 cursor-pointer`;

// State
const fileStore = useFileStore();
const walletStore = useWalletStore();
const { pendingFilesSignature } = storeToRefs(fileStore);

const { openConnectWallet, openDisconnectWallet, wallet } =
  storeToRefs(walletStore);
const notifyType = ref<'info' | 'warning'>('info');
const notifyTitle = ref('');
const notifyDetails = ref('');
const notifyCancelEnabled = ref(false);
const showNotification = ref(false);
const isFadeOut = ref(false);
const removeSplashScreen = ref(false);

// Methods
const handleClickUpload = async () => {
  try {
    if (wallet.value.isConnected) {
      await navigateTo('/upload');
    } else {
      walletStore.showConnectWallet(async () => {
        await navigateTo('/upload');
      });
    }
  } catch (error) {
    // TODO: Handle error
    console.error('>>> Error: handleClickUpload');
  }
};

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
        'To view your files please sign the file request in your wallet.',
        enabledCancel: true,
    });
  } else {
    handleHideNotification();
  }
});

onMounted(async () => {
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

            <NuxtLink :class="`${classesLinks}`" to="/settings">
              <IconSettings class="w-6 h-6" />
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
          <Toast position="bottom-right" />
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

        <PaymentDrawer
          @show-notify="handleShowNotification"
          @hide-notify="handleHideNotification"
        />
      </div>
    </NuxtLayout>
  </div>
</template>
