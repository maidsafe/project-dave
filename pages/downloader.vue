<script lang="ts" setup>
import {ref, computed} from "vue";
import {invoke} from "@tauri-apps/api/core";
import {useToast} from "primevue/usetoast";
import {useWalletStore} from "~/stores/wallet";
import {storeToRefs} from "pinia";

const emit = defineEmits(["show-notify", "hide-notify"]);

const toast = useToast();
const walletStore = useWalletStore();
const {wallet} = storeToRefs(walletStore);

const inputValue = ref("");
const customFileName = ref("");
const isDownloading = ref(false);
const downloadedFile = ref<{ path: string; fileName: string; inputType: string; originalInput: string } | null>(null);
const isAddingToVault = ref(false);

const inputType = computed(() => {
  const trimmed = inputValue.value.trim();
  if (!trimmed) return null;

  // Remove 0x prefix if present for validation
  const cleanHex = trimmed.toLowerCase().startsWith("0x") ? trimmed.substring(2) : trimmed;

  // Check if it's a valid hex string (case insensitive)
  if (!/^[0-9a-fA-F]+$/i.test(cleanHex)) {
    return null;
  }

  // Data address is exactly 64 hex characters
  if (cleanHex.length === 64) {
    return "address";
  }
  // Data map hex is typically longer than 64 characters
  else if (cleanHex.length > 64) {
    return "datamap";
  }

  return null;
});

const isValidInput = computed(() => inputType.value !== null && customFileName.value.trim() !== "");

const getFileName = () => {
  return customFileName.value.trim();
};

const downloadFile = async () => {
  if (!isValidInput.value || isDownloading.value) return;

  try {
    isDownloading.value = true;
    downloadedFile.value = null;

    const appData = await invoke("app_data") as any;
    const downloadPath = appData.download_path || "";

    if (!downloadPath) {
      toast.add({
        severity: "error",
        summary: "Error",
        detail: "Please set download directory in settings first",
        life: 5000
      });
      return;
    }

    const fileName = getFileName();
    const destinationPath = await invoke("get_unique_download_path", {
      downloadsPath: downloadPath,
      filename: fileName
    }) as string;

    if (inputType.value === "address") {
      let address = inputValue.value.trim();
      // Remove 0x prefix if present for addresses too
      if (address.startsWith("0x")) {
        address = address.substring(2);
      }

      await invoke("download_public_file", {
        addr: address,
        toDest: destinationPath
      });
    } else if (inputType.value === "datamap") {
      let dataMapHex = inputValue.value.trim();
      if (dataMapHex.startsWith("0x")) {
        dataMapHex = dataMapHex.substring(2);
      }

      const dataMapBytes = Array.from(
          dataMapHex.match(/.{1,2}/g) || [],
          byte => parseInt(byte, 16)
      );

      await invoke("download_private_file", {
        dataMapChunk: dataMapBytes,
        toDest: destinationPath
      });
    }

    downloadedFile.value = {
      path: destinationPath,
      fileName: destinationPath.split(/[\\/]/).pop() || fileName,
      inputType: inputType.value!,
      originalInput: inputValue.value.trim()
    };

    toast.add({
      severity: "success",
      summary: "Success",
      detail: "Download completed successfully",
      life: 3000
    });
  } catch (error) {
    console.error("Download error:", error);
    toast.add({
      severity: "error",
      summary: "Download Failed",
      detail: error instanceof Error ? error.message : "Failed to download file",
      life: 5000
    });
  } finally {
    isDownloading.value = false;
  }
};

const showInFolder = async () => {
  if (!downloadedFile.value) return;

  try {
    await invoke("show_item_in_file_manager", {
      path: downloadedFile.value.path
    });
  } catch (error) {
    console.error("Failed to show file:", error);
    toast.add({
      severity: "error",
      summary: "Error",
      detail: "Failed to show file in folder",
      life: 3000
    });
  }
};

const addDirectlyToVault = async () => {
  if (!isValidInput.value || !wallet.value.isConnected || isAddingToVault.value) return;

  try {
    isAddingToVault.value = true;

    // Get vault key signature
    const vaultKeySignature = await walletStore.getVaultKeySignature();

    // Get the file name
    const fileName = getFileName();

    // Show notification that we're adding the file
    emit('show-notify', {
      notifyType: 'info',
      title: 'Adding file to vault',
      details: `Adding "${fileName}" to your vault...`,
      enabledCancel: false
    });

    // Determine the file access type
    let fileAccess;
    if (inputType.value === "address") {
      let address = inputValue.value.trim();
      if (address.startsWith("0x")) {
        address = address.substring(2);
      }
      fileAccess = {Public: address};
    } else {
      let dataMapHex = inputValue.value.trim();
      if (dataMapHex.startsWith("0x")) {
        dataMapHex = dataMapHex.substring(2);
      }
      const dataMapBytes = Array.from(
          dataMapHex.match(/.{1,2}/g) || [],
          byte => parseInt(byte, 16)
      );
      fileAccess = {Private: dataMapBytes};
    }

    // Add the file to vault using the Tauri command
    await invoke("add_local_file_to_vault", {
      vaultKeySignature: vaultKeySignature,
      fileAccess: fileAccess,
      fileName: fileName
    });

    // Hide the notification
    emit('hide-notify');

    toast.add({
      severity: "success",
      summary: "Added to Vault",
      detail: `File "${fileName}" has been added to your vault.`,
      life: 3000
    });

    // Reset after successful vault addition
    reset();

  } catch (error) {
    console.error("Failed to add to vault:", error);

    // Hide the notification on error too
    emit('hide-notify');

    toast.add({
      severity: "error",
      summary: "Failed to add to vault",
      detail: error instanceof Error ? error.message : "Failed to add file to vault",
      life: 5000
    });
  } finally {
    isAddingToVault.value = false;
  }
};

const reset = () => {
  inputValue.value = "";
  customFileName.value = "";
  downloadedFile.value = null;
};
</script>

<template>
  <div class="px-[66px] lg:px-[110px] pt-[70px] pb-10">
    <h1 class="text-3xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-2">
      Downloader
    </h1>
    <p class="text-autonomi-text-primary mb-8">
      Download files from the Autonomi network.
    </p>

    <div class="bg-white dark:bg-white/10 rounded-lg p-6 shadow-sm">
      <div class="space-y-6">
        <!-- Input Section -->
        <div>
          <label class="block text-sm font-medium text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-2">
            Data Address or Data Map Hex <span class="text-red-500">*</span>
          </label>
          <InputText
              v-model="inputValue"
              :disabled="isDownloading"
              placeholder="Enter data address or data map hex (e.g., 0x...)"
              class="w-full"
          />
          <p v-if="inputValue && !inputType" class="mt-2 text-sm text-red-500">
            Please enter a valid data address or data map hex
          </p>
          <p v-else-if="inputType" class="mt-2 text-sm text-autonomi-text-secondary">
            Detected type: {{ inputType === 'address' ? 'Data Address' : 'Data Map Hex' }}
          </p>
        </div>

        <!-- Custom File Name Section -->
        <div>
          <label class="block text-sm font-medium text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-2">
            File Name <span class="text-red-500">*</span>
          </label>
          <InputText
              v-model="customFileName"
              :disabled="isDownloading"
              placeholder="Enter file name (e.g., myfile.txt)"
              class="w-full"
          />
          <p v-if="!customFileName.trim() && inputValue" class="mt-2 text-sm text-red-500">
            Please enter a file name
          </p>
        </div>

        <!-- Download and Action Buttons -->
        <div class="flex gap-3 items-center">
          <button
              @click="downloadFile"
              :disabled="!isValidInput || isDownloading"
              class="px-7 py-2 rounded-full text-sm transition-all duration-300"
              :class="{
              'bg-autonomi-red-300 text-white hover:bg-autonomi-red-300/80': isValidInput && !isDownloading,
              'bg-gray-300 text-gray-500 cursor-not-allowed': !isValidInput || isDownloading
            }"
          >
            {{ isDownloading ? 'Downloading...' : 'Download' }}
          </button>

          <button
              v-if="wallet.isConnected"
              @click="addDirectlyToVault"
              :disabled="!isValidInput || isAddingToVault"
              class="px-7 py-2 rounded-full text-sm transition-all duration-300"
              :class="{
              'bg-sky-500 text-white hover:bg-sky-500/80': isValidInput && !isAddingToVault,
              'bg-gray-300 text-gray-500 cursor-not-allowed': !isValidInput || isAddingToVault
            }"
          >
            {{ isAddingToVault ? 'Adding...' : 'Add to Vault' }}
          </button>

          <CommonButton
              v-if="downloadedFile"
              variant="tertiary"
              size="medium"
              @click="reset"
          >
            Reset
          </CommonButton>
        </div>

        <!-- Downloaded File Section -->
        <div v-if="downloadedFile" class="mt-8 p-4 bg-autonomi-primary/10 dark:bg-autonomi-primary/20 rounded-lg">
          <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-3">
            Download Complete
          </h3>

          <p class="text-sm text-autonomi-text-primary mb-4">
            <span class="font-medium">File:</span> {{ downloadedFile.fileName }}
          </p>

          <div class="flex gap-3">
            <CommonButton
                variant="secondary"
                size="small"
                @click="showInFolder"
            >
              Show in Folder
            </CommonButton>
          </div>
        </div>

        <!-- Wallet Not Connected Message -->
        <div v-if="downloadedFile && !wallet.isConnected"
             class="mt-4 p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg">
          <p class="text-sm text-yellow-800 dark:text-yellow-200">
            Connect your wallet to add downloaded files to your vault
          </p>
        </div>
      </div>
    </div>
  </div>
</template>