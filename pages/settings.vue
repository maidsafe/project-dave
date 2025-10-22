<script lang="ts" setup>
import {ref, onMounted} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import {open} from '@tauri-apps/plugin-dialog';
import {useToast} from 'primevue/usetoast';

const toast = useToast();

// State
const downloadDirectory = ref<string>('');
const isLoading = ref(false);
const isSaving = ref(false);
const appVersion = ref<string>('');
const usePaymaster = ref<boolean>(false);

// Load current settings
const loadSettings = async () => {
  try {
    isLoading.value = true;
    const appData = await invoke('app_data') as any;
    downloadDirectory.value = appData.download_path || '';
    usePaymaster.value = appData.use_paymaster ?? false;
  } catch (error) {
    console.error('Failed to load settings:', error);
    toast.add({
      severity: 'error',
      summary: 'Error',
      detail: 'Failed to load settings',
      life: 3000
    });
  } finally {
    isLoading.value = false;
  }
};

// Choose directory and auto-save
const chooseDirectory = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Choose Download Directory'
    });

    if (selected && selected !== downloadDirectory.value) {
      const previousValue = downloadDirectory.value;
      downloadDirectory.value = selected as string;

      // Auto-save the new directory
      try {
        isSaving.value = true;
        const currentAppData = await invoke('app_data') as any;
        const updatedAppData = {
          ...currentAppData,
          download_path: downloadDirectory.value
        };

        await invoke('app_data_store', {
          appData: updatedAppData
        });


        toast.add({
          severity: 'success',
          summary: 'Success',
          detail: 'Download directory updated',
          life: 3000
        });
      } catch (saveError) {
        // Revert on error
        downloadDirectory.value = previousValue;
        console.error('Failed to save directory:', saveError);
        toast.add({
          severity: 'error',
          summary: 'Error',
          detail: 'Failed to save download directory',
          life: 3000
        });
      } finally {
        isSaving.value = false;
      }
    }
  } catch (error) {
    console.error('Failed to select directory:', error);
    toast.add({
      severity: 'error',
      summary: 'Error',
      detail: 'Failed to select directory',
      life: 3000
    });
  }
};

// Open logs folder
const openLogsFolder = async () => {
  try {
    const logsPath = await invoke('get_logs_directory');
    await invoke('show_item_in_file_manager', {path: logsPath});
  } catch (error) {
    console.error('Failed to open logs folder:', error);
    toast.add({
      severity: 'error',
      summary: 'Error',
      detail: 'Failed to open logs folder',
      life: 3000
    });
  }
};

// Auto-save paymaster settings when toggled
const onPaymasterToggle = async () => {
  const previousValue = !usePaymaster.value; // Store the opposite of current value

  try {
    isSaving.value = true;
    const currentAppData = await invoke('app_data') as any;
    const updatedAppData = {
      ...currentAppData,
      use_paymaster: usePaymaster.value
    };

    await invoke('app_data_store', {
      appData: updatedAppData
    });

    toast.add({
      severity: 'success',
      summary: 'Success',
      detail: usePaymaster.value
        ? 'Paymaster enabled - gas-free transactions active'
        : 'Paymaster disabled - standard payments active',
      life: 3000
    });
  } catch (error) {
    // Revert on error
    usePaymaster.value = previousValue;
    console.error('Failed to save paymaster settings:', error);
    toast.add({
      severity: 'error',
      summary: 'Error',
      detail: 'Failed to update paymaster settings',
      life: 3000
    });
  } finally {
    isSaving.value = false;
  }
};


onMounted(async () => {
  loadSettings();

  // Get app version
  try {
    appVersion.value = await invoke('get_app_version');
  } catch (error) {
    console.error('Failed to get app version:', error);
  }
});
</script>

<template>
  <div class="px-[66px] lg:px-[110px] pt-[70px] pb-10">
    <h1 class="text-3xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-2">
      Settings
    </h1>
    <p class="text-autonomi-text-primary mb-8">
      Configure your preferences and download settings.
    </p>

    <div v-if="isLoading" class="flex items-center justify-center py-20">
      <ProgressSpinner/>
    </div>

    <div v-else class="bg-white dark:bg-white/10 rounded-lg p-6 shadow-sm">
      <div class="space-y-6">
        <!-- Download Directory Section -->
        <div>
          <h2 class="text-xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
            Download Directory
          </h2>
          <p class="text-sm text-autonomi-text-primary mb-4">
            Choose where your downloaded files will be saved.
          </p>

          <div class="flex gap-3 items-center">
            <div class="flex-1">
              <InputText
                  v-model="downloadDirectory"
                  :disabled="true"
                  placeholder="No directory selected"
                  class="w-full"
              />
            </div>
            <CommonButton
                variant="secondary"
                size="medium"
                @click="chooseDirectory"
                :disabled="isSaving"
                :loading="isSaving"
            >
              Browse...
            </CommonButton>
          </div>

          <div v-if="downloadDirectory" class="mt-2">
            <p class="text-sm text-autonomi-text-secondary dark:text-autonomi-text-secondary-dark">
              Current: {{ downloadDirectory }}
            </p>
          </div>
        </div>

        <!-- Paymaster Settings Section -->
        <div class="border-t border-white/10 pt-6">
          <h2 class="text-xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
            Paymaster Settings
          </h2>
          <p class="text-sm text-autonomi-text-primary mb-4">
            Enable paymaster to pay for transactions using only ANT tokens without needing ETH for gas fees.
          </p>

          <div class="flex items-center gap-3">
            <Checkbox
              v-model="usePaymaster"
              inputId="usePaymaster"
              binary
              @change="onPaymasterToggle"
              :disabled="isSaving"
            />
            <label for="usePaymaster" class="text-sm text-autonomi-text-primary cursor-pointer">
              Enable Paymaster (Gas-free transactions)
            </label>
          </div>
        </div>

        <!-- Logs Directory Section -->
        <div class="border-t border-white/10 pt-6">
          <h2 class="text-xl font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
            Application Logs
          </h2>
          <p class="text-sm text-autonomi-text-primary mb-4">
            View application logs for troubleshooting and debugging.
          </p>

          <CommonButton
              variant="secondary"
              size="medium"
              @click="openLogsFolder"
          >
            Open Logs Folder
          </CommonButton>
        </div>
      </div>
    </div>

    <!-- Version info at the bottom -->
    <div class="mt-8 text-center">
      <p class="text-sm text-autonomi-text-secondary dark:text-autonomi-text-secondary-dark">
        Dave version {{ appVersion }}
      </p>
    </div>
  </div>
</template>