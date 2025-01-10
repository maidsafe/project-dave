<script lang="ts" setup>
// Header Component
import { ref } from "vue";
import Button from "primevue/button";
import IconField from "primevue/iconfield";
import InputIcon from "primevue/inputicon";
import { useWalletStore } from "~/stores/wallet";
import { useUserStore } from "~/stores/user";
import { usePaymentStore } from "~/stores/payments";
import type { PaymentOrder } from "~/stores/payments";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { storeToRefs } from "pinia";

const value1 = ref("");
const searching = ref(false);
const walletStore = useWalletStore();
const { wallet } = storeToRefs(walletStore);
const userStore = useUserStore();
const paymentStore = usePaymentStore();
const {
  IDLE_PAYMENT_EXPIRATION_TIME_SECS,
  pendingPayments,
  pendingPaymentsCount,
} = storeToRefs(paymentStore);

// Refs
const refTokenDropdown = ref();
const isTokenDropdownOpen = ref(false);
const refHamburgerMenu = ref();
const isHamburgerMenuOpen = ref(false);

// Methods
const handleClickWallet = () => {
  if (wallet.value.isConnected) {
    walletStore.showDisconnectWallet();
  } else {
    walletStore.showConnectWallet();
  }
};

const handleClickUpload = ()=> {
  if (wallet.value.isConnected) {
    navigateTo('/upload');
  } else {
    walletStore.showConnectWallet(() => {
      navigateTo('/upload');
    });
  }
};

const toggleTokenDropdown = () => {
  if (!wallet.value.isConnected) {
    return walletStore.showConnectWallet();
  }
  // Hide hamburger if open
  isHamburgerMenuOpen.value = false;

  // Toggle dropdown
  isTokenDropdownOpen.value = !isTokenDropdownOpen.value;
};

const toggleHamburgerMenu = () => {
  if (!wallet.value.isConnected) {
    return walletStore.showConnectWallet();
  }

  // Hide token dropdown if open
  isTokenDropdownOpen.value = false;

  // Toggle hamburger
  isHamburgerMenuOpen.value = !isHamburgerMenuOpen.value;
};

onMounted(() => {
  // Close dropdown on click outside
  document.addEventListener("click", (event) => {
    if (
      refTokenDropdown.value &&
      !refTokenDropdown.value.contains(event.target)
    ) {
      isTokenDropdownOpen.value = false;
    }

    if (
      refHamburgerMenu.value &&
      !refHamburgerMenu.value.contains(event.target)
    ) {
      isHamburgerMenuOpen.value = false;
    }
  });

  listen<PaymentOrder>("payment-order", async (event: any) => {
    let order: PaymentOrder = JSON.parse(event.payload);

    if (!order) return;

    console.log(">>> PAYMENT ORDER RECEIVED HEADER", order);

    paymentStore.addPendingPayment(order.id, {
      order,
      expires: Date.now() + 1000 * IDLE_PAYMENT_EXPIRATION_TIME_SECS.value,
      processing: ProcessingState.PENDING,
    });

    console.log(
      ">>> LISTENING PAYMENT ORDER FROM HEADER:",
      pendingPayments.value
    );
  });
});

onBeforeUnmount(() => {
  document.removeEventListener("click", () => {});
});
</script>

<template>
  <div>
    <div
      class="sticky top-0 min-h-[68px] lg:min-h-[110px] bg-autonomi-gray-50 dark:bg-autonomi-blue-600 transition-all duration-300"
    >
      <div
        class="flex items-center px-4 lg:px-[62px] h-[68px] lg:h-[110px] bg-white lg:bg-autonomi-gray-50 lg:dark:bg-autonomi-blue-600 rounded-b-2xl"
      >
        <div class="flex gap-2 cursor-pointer" @click="navigateTo('/')">
          <IconLogo :dark="false" />
          <p class="text-autonomi-text-primary">Beta</p>
        </div>

        <!-- SEARCH -->
        <div
          :class="`ml-[116px] mr-10 max-w-[672px] transition-all duration-1000 overflow-hidden hidden lg:block ${
            wallet.isConnected ? 'w-full' : 'w-0'
          }`"
        >
          <IconField v-if="searching">
            <InputIcon class="pi pi-spin pi-spinner" />
            <InputText
              v-model="value1"
              variant="filled"
              class="w-full"
              disabled
            />
          </IconField>
          <IconField v-else>
            <InputIcon class="pi pi-search" />
            <InputText
              v-model="userStore.query"
              placeholder="Search"
              class="w-full leading-[34px] !rounded-3xl"
            />
          </IconField>
        </div>

        <!-- SETTINGS / DETAILS -->
        <div class="ml-auto hidden lg:flex items-center">
          <div class="flex items-center gap-3">
            <div
              v-tooltip.bottom="
                wallet.isConnected ? 'Disconnect Mobile Wallet' : 'Connect Mobile Wallet'
              "
              class="w-10 h-10 rounded-full bg-autonomi-blue-800 flex items-center justify-center cursor-pointer"
              @click="handleClickWallet"
            >
              <svg
                width="22"
                height="22"
                viewBox="0 0 22 22"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
              >
                <rect width="21.16" height="21.16" fill="#26264C" />
                <path
                  d="M16.752 6.1712H15.8703V5.28953C15.8703 4.58803 15.5917 3.91527 15.0956 3.41923C14.5996 2.9232 13.9268 2.64453 13.2253 2.64453H4.40867C3.70717 2.64453 3.03441 2.9232 2.53837 3.41923C2.04234 3.91527 1.76367 4.58803 1.76367 5.28953V15.8695C1.76367 16.571 2.04234 17.2438 2.53837 17.7398C3.03441 18.2359 3.70717 18.5145 4.40867 18.5145H16.752C17.4535 18.5145 18.1263 18.2359 18.6223 17.7398C19.1183 17.2438 19.397 16.571 19.397 15.8695V8.8162C19.397 8.1147 19.1183 7.44193 18.6223 6.9459C18.1263 6.44987 17.4535 6.1712 16.752 6.1712ZM4.40867 4.40786H13.2253C13.4592 4.40786 13.6834 4.50075 13.8488 4.6661C14.0141 4.83144 14.107 5.0557 14.107 5.28953V6.1712H4.40867C4.17484 6.1712 3.95058 6.07831 3.78524 5.91296C3.61989 5.74762 3.527 5.52336 3.527 5.28953C3.527 5.0557 3.61989 4.83144 3.78524 4.6661C3.95058 4.50075 4.17484 4.40786 4.40867 4.40786ZM17.6337 13.2245H16.752C16.5182 13.2245 16.2939 13.1316 16.1286 12.9663C15.9632 12.801 15.8703 12.5767 15.8703 12.3429C15.8703 12.109 15.9632 11.8848 16.1286 11.7194C16.2939 11.5541 16.5182 11.4612 16.752 11.4612H17.6337V13.2245ZM17.6337 9.69786H16.752C16.0505 9.69786 15.3777 9.97653 14.8817 10.4726C14.3857 10.9686 14.107 11.6414 14.107 12.3429C14.107 13.0444 14.3857 13.7171 14.8817 14.2132C15.3777 14.7092 16.0505 14.9879 16.752 14.9879H17.6337V15.8695C17.6337 16.1034 17.5408 16.3276 17.3754 16.493C17.2101 16.6583 16.9858 16.7512 16.752 16.7512H4.40867C4.17484 16.7512 3.95058 16.6583 3.78524 16.493C3.61989 16.3276 3.527 16.1034 3.527 15.8695V7.78465C3.81026 7.88429 4.10841 7.93498 4.40867 7.93453H16.752C16.9858 7.93453 17.2101 8.02742 17.3754 8.19277C17.5408 8.35811 17.6337 8.58237 17.6337 8.8162V9.69786Z"
                  fill="white"
                />
              </svg>
            </div>
            <template v-if="address">
              <div
                :class="`${
                  wallet.isConnected ? 'w-[150px]' : 'w-0'
                } transition-all duration-500 overflow-hidden whitespace-nowrap`"
              >
                <div class="text-sm font-semibold text-autonomi-text-secondary">
                  {{ `${address.slice(0, 6)}...${address.slice(38, 42)}` }}
                </div>
                <div class="text-sm text-autonomi-text-primary">
                  ANT (unknown)
                </div>
              </div>
            </template>

            <!-- PAYMENT ORDERS -->
            <!-- <PaymentOrders/> -->
            <Button
              class="hidden lg:flex gap-1"
              type="button"
              icon="pi pi-receipt"
              label="Pay"
              :severity="pendingPaymentsCount > 0 ? 'info' : 'secondary'"
              :badge="pendingPaymentsCount.toString()"
              @click="paymentStore.openPaymentDrawer()"
            />
          </div>
        </div>

        <!-- MOBILE SETTINGS / DETAILS -->
        <div class="ml-auto flex gap-2 items-center lg:hidden">
          <NuxtLink
            to="/"
            class="w-10 h-10 bg-autonomi-gray-200 flex items-center justify-center rounded-full cursor-pointer"
            v-tooltip.bottom="'Files'"
           >
           <IconFiles class="w-5 h-5" />
          </NuxtLink>
          <div
          @click="handleClickUpload"
            class="w-10 h-10 bg-autonomi-red-300 flex items-center justify-center rounded-full cursor-pointer"
            v-tooltip.bottom="'Upload'"
          >
            <i class="pi pi-plus-circle text-white" />
        </div>

          <!-- TOKEN BALANCE DROPDOWN -->
          <div ref="refTokenDropdown">
            <button
              class="w-10 h-10 bg-autonomi-blue-800 flex items-center justify-center rounded-full cursor-pointer"
              @click="handleClickWallet"
            >
              <i class="pi pi-wallet text-white" />
            </button>

            <div
              class="bg-autonomi-gray-200 dark:bg-autonomi-blue-800 absolute top-[150px] right-0 w-full max-w-[400px] rounded-b-3xl overflow-hidden transition-all duration-500"
              :class="{
                'h-0': !isTokenDropdownOpen,
                'h-[100px]': isTokenDropdownOpen,
                'top-[150px]': wallet.isConnected,
                'top-[68px]': !wallet.isConnected,
              }"
            >
              <div
                class="h-[100px] flex gap-3 items-center justify-start px-10 border-t-2 border-t-white hover:bg-white transition-all duration-500"
              >
                <div
                  class="w-10 h-10 bg-autonomi-blue-800 flex items-center justify-center rounded-full cursor-pointer"
                  v-tooltip.bottom="'Disconnect Mobile Wallet'"
                  @click="handleClickWallet"
                >
                  <i class="pi pi-wallet text-white" />
                </div>
                <div>
                  <div
                    class="text-sm font-semibold text-autonomi-text-secondary"
                  >
                    {{ account }}
                  </div>
                  <div class="text-sm text-autonomi-text-primary">
                    Balance $0.00
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- PAYMENT ORDERS -->
          <!-- MOBILE -->
          <Button
            class="flex lg:hidden gap-1"
            type="button"
            icon="pi pi-receipt"
            :severity="pendingPaymentsCount > 0 ? 'info' : 'secondary'"
            :badge="pendingPaymentsCount.toString()"
            @click="paymentStore.openPaymentDrawer()"
          />

          <!-- HAMBURGER MENU -->
          <div ref="refHamburgerMenu">
            <button
              class="w-10 h-10 bg-autonomi-gray-50 flex items-center justify-center rounded-full cursor-pointer"
              @click="toggleHamburgerMenu"
            >
              <i class="pi pi-bars text-autonomi-blue-800" />
            </button>

            <div
              class="bg-autonomi-gray-200 dark:bg-autonomi-blue-800 absolute right-0 w-full max-w-[400px] rounded-b-3xl overflow-hidden transition-all duration-500"
              :class="{
                'h-0': !isHamburgerMenuOpen,
                'h-[100px]': isHamburgerMenuOpen,
                'top-[150px]': wallet.isConnected,
                'top-[68px]': !wallet.isConnected,
              }"
            >
              <div
                @click="
                  () => {
                    toggleHamburgerMenu();
                    navigateTo('/settings');
                  }
                "
                class="h-[100px] flex items-center justify-between px-10 border-t-2 border-t-white hover:bg-white transition-all duration-300 cursor-pointer"
              >
                <div>
                  <div class="text-2xl font-semibold text-autonomi-header-text">
                    About us
                  </div>
                  <!-- <div class="text-xs font-semibold text-autonomi-gray-500">
                    Current: Default
                  </div> -->
                </div>
                <div>
                  <IconSettings class="w-12 h-12" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- MOBILE SEARCH -->
      <div
        :class="`px-[30px] lg:px-[62px] ${
          wallet.isConnected ? 'w-full' : 'hidden'
        } lg:hidden`"
      >
        <!-- SEARCH -->
        <div
          :class="`py-3 max-w-[672px] transition-all duration-1000 overflow-hidden mx-auto ${
            wallet.isConnected ? 'w-full' : 'hidden'
          }`"
        >
          <IconField v-if="searching">
            <InputIcon class="pi pi-spin pi-spinner" />
            <InputText
              v-model="value1"
              variant="filled"
              class="w-full"
              disabled
            />
          </IconField>
          <IconField v-else>
            <InputIcon class="pi pi-search" />
            <InputText
              v-model="userStore.query"
              placeholder="Search"
              class="w-full leading-[34px] !rounded-3xl"
            />
          </IconField>
        </div>
      </div>
    </div>
  </div>
</template>
