<script lang="ts" setup>
import {useFileStore} from '~/stores/files';
import {useToast} from 'primevue/usetoast';
import {useUserStore} from '~/stores/user';
import {useUploadStore} from '~/stores/upload';
import {useUploadsStore} from '~/stores/uploads';
import {useDownloadsStore} from '~/stores/downloads';
import {useWalletStore} from '~/stores/wallet';
import {usePaymentStore} from '~/stores/payments';
import {invoke} from '@tauri-apps/api/core';
import {downloadDir} from '@tauri-apps/api/path';
import {open} from '@tauri-apps/plugin-dialog';
import {basename} from '@tauri-apps/api/path';

const toast = useToast();
const fileStore = useFileStore();
const uploadStore = useUploadStore();
const uploadsStore = useUploadsStore();
const downloadsStore = useDownloadsStore();
const walletStore = useWalletStore();
const paymentStore = usePaymentStore();

const {
  pendingVaultStructure,
  currentDirectory,
  currentDirectoryFiles,
  rootDirectory,
  files,
  failedArchives,
} = storeToRefs(fileStore);
const {uploadProgress} = storeToRefs(uploadStore);
const userStore = useUserStore();
const {query} = storeToRefs(userStore);

const activeTab = ref(0); // 0: Files, 1: Uploads, 2: Downloads
const viewTypeVault = ref<'grid' | 'list'>('list');
const breadcrumbs = ref<any[]>([]);
const isVisibleFileInfo = ref(false);
const refFilesMenu = ref();
const refFilesViewMenu = ref();
const refDownloadMenu = ref();
const refUploadMenu = ref();
const selectedDownloadItem = ref<any>();
const selectedFileItem = ref<any>();
const selectedUploadItem = ref<any>();
const showUploadModal = ref(false);
const uploadSteps = ref<any[]>([]);
const currentUploadStep = ref<string>('');
const quoteData = ref<any>(null);
const uploadError = ref<string>('');
const pendingUploadFiles = ref<any>(null);
const currentUploadId = ref<string | null>(null);

// Upload modal functionality
const initializeUploadSteps = () => {
  uploadSteps.value = [
    {
      key: 'processing',
      label: 'Processing Files',
      status: 'pending',
      message: 'Reading and preparing files...'
    },
    {
      key: 'encrypting',
      label: 'Encrypting',
      status: 'pending',
      message: 'Encrypting files for secure storage...'
    },
    {
      key: 'quoting',
      label: 'Getting Quote',
      status: 'pending',
      message: 'Calculating storage costs...'
    },
    {
      key: 'payment-request',
      label: 'Payment Request',
      status: 'pending',
      message: 'Requesting payment authorization...'
    }
  ];
  currentUploadStep.value = '';
  uploadError.value = '';
  quoteData.value = null;
};

// Update step status
const updateStepStatus = (stepKey: string, status: string, message?: string, progress?: number) => {
  const step = uploadSteps.value.find(s => s.key === stepKey);
  if (step) {
    step.status = status;
    if (message) step.message = message;
    if (progress !== undefined) step.progress = progress;
  }
  currentUploadStep.value = stepKey;
};

const handleCancelUploadModal = () => {
  showUploadModal.value = false;
  pendingUploadFiles.value = null;
  uploadSteps.value = [];
  currentUploadStep.value = '';
  uploadError.value = '';
  quoteData.value = null;
  currentUploadId.value = null;
};

const handleCloseUploadModal = () => {
  const hasActiveProcessing = uploadSteps.value.some(step => step.status === 'processing');
  if (!hasActiveProcessing) {
    handleCancelUploadModal();
  }
};

const handlePayUpload = async () => {
  if (!quoteData.value?.paymentOrderId || !quoteData.value?.rawQuoteData?.payments) {
    console.error("No payment order ID or payments data available");
    return;
  }

  try {
    console.log(">>> Processing payment for order:", quoteData.value.paymentOrderId);
    console.log(">>> Payment data:", quoteData.value.rawQuoteData.payments);

    // Update UI to show payment in progress
    updateStepStatus('payment-request', 'processing', 'Requesting wallet authorization...');

    // Process payment through wallet
    const txHashes = await walletStore.payForQuotes(quoteData.value.rawQuoteData.payments);
    console.log(">>> Payment successful, transaction hashes:", txHashes);

    // Update UI to show wallet payment successful
    updateStepStatus('payment-request', 'processing', 'Payment confirmed, notifying backend...');

    // Send payment confirmation to backend
    await invoke("confirm_payment", {
      orderId: quoteData.value.paymentOrderId
    });

    // Update UI to show payment completed
    updateStepStatus('payment-request', 'completed', 'Payment completed');

  } catch (error) {
    console.error("Payment failed:", error);
    updateStepStatus('payment-request', 'error', 'Payment failed');

    let errorMessage = 'Could not process payment. Please try again.';
    let errorSummary = 'Payment Failed';

    if (error && typeof error === 'object' && 'message' in error && typeof error.message === 'string') {
      if (error.message.includes('User rejected') || error.message.includes('user rejected')) {
        errorSummary = 'Payment Cancelled';
        errorMessage = 'Payment was cancelled by user.';
      } else if (error.message.includes('insufficient')) {
        errorSummary = 'Insufficient Funds';
        errorMessage = 'Insufficient tokens to complete payment.';
      } else {
        errorMessage = error.message;
      }
    }

    toast.add({
      severity: 'error',
      summary: errorSummary,
      detail: errorMessage,
      life: 5000,
    });
  }
};

// File info handlers
const handleFileNameClick = (file: any) => {
  // Only open info for files, not folders
  if (file.path) {
    selectedFileItem.value = file;
    isVisibleFileInfo.value = true;
  }
};

// Menu definitions
const menuFiles = computed(() => {
  const file = selectedFileItem.value;
  const items = [];

  if (file?.is_loading) {
    items.push({
      label: 'Loading...',
      icon: 'pi pi-spinner pi-spin',
      disabled: true,
    });
  } else if (file?.load_error) {
    items.push({
      label: 'Retry Download',
      icon: 'pi pi-refresh',
      command: () => handleDownloadFile(file),
    });
  } else if (file?.path) {
    // Only show download for files (not folders)
    items.push({
      label: 'Download',
      icon: 'pi pi-download',
      command: () => handleDownloadFile(file),
    });
  }

  if (file?.path) {
    // Only show info for files (not folders)
    items.push({
      label: 'Info',
      icon: 'pi pi-info-circle',
      command: () => {
        isVisibleFileInfo.value = true;
      },
    });
  }

  return items;
});

const menuUploads = ref([
  {
    label: 'Remove',
    icon: 'pi pi-times',
    command: () => {
      if (selectedUploadItem.value) {
        uploadsStore.removeUpload(selectedUploadItem.value.id);
      }
    },
  },
]);

const menuDownloads = ref([
  {
    label: 'Remove',
    icon: 'pi pi-times',
    command: () => {
      if (selectedDownloadItem.value) {
        downloadsStore.removeDownload(selectedDownloadItem.value.id);
      }
    },
  },
]);

// View functions
const handleShowListView = () => {
  viewTypeVault.value = 'list';
};

const handleShowGridView = () => {
  viewTypeVault.value = 'grid';
};

const menuFilesView = ref([
  {
    label: 'List',
    icon: 'pi pi-list',
    command: handleShowListView,
  },
  {
    label: 'Grid',
    icon: 'pi pi-th-large',
    command: handleShowGridView,
  },
]);

// Upload functions
const emit = defineEmits(["show-notify", "hide-notify"]);
const isUploading = computed(() => uploadStore.uploadProgress.isUploading);

const filteredFiles = computed(() => {
  try {
    if (!currentDirectory.value?.children?.length) {
      return [];
    }

    return currentDirectory.value.children.filter((folder: any) => {
      if (query.value) {
        return (
            folder.name.toLowerCase().includes(query.value.toLowerCase()) &&
            folder.name !== 'parents'
        );
      }

      return folder.name !== 'parents';
    });
  } catch (error) {
    return [];
  }
});

// Combine regular files and failed archives
const combinedFiles = computed(() => {
  const regularFiles = filteredFiles.value || [];
  const failedArchiveFiles = failedArchives.value.map(archive => ({
    name: archive.name,
    is_failed_archive: true,
    is_private: archive.is_private,
    is_loaded: false,
    is_loading: false,
    load_error: true,
    path: `failed-archive://${archive.name}`,
    metadata: {}
  }));

  return [...regularFiles, ...failedArchiveFiles];
});

// Computed for loading progress
const loadingProgress = computed(() => {
  if (!files.value.length) return {loaded: 0, total: 0, loading: 0, errors: 0};

  const loaded = files.value.filter(f => f.is_loaded).length;
  const loading = files.value.filter(f => f.is_loading).length;
  const errors = files.value.filter(f => f.load_error).length;
  const total = files.value.length;

  return {loaded, total, loading, errors};
});

const openPickerAndUploadFiles = async () => {
  let selected = await open({multiple: true});
  if (selected === null) return;
  if (!Array.isArray(selected)) selected = [selected];
  
  const files = await Promise.all(
      selected.map(async (file) => {
        return {path: file, name: await basename(file)};
      })
  );

  await uploadFiles(files);
};

const openFolderPickerAndUploadFiles = async () => {
  const selected = await open({directory: true});
  if (selected === null) return;
  
  const files = [{path: selected, name: await basename(selected)}];
  await uploadFiles(files);
};

const uploadFiles = async (files: Array<{ path: string, name: string }>) => {
  try {
    console.log(">>> FILEVIEWER GETTING VAULT KEY SIGNATURE");
    emit("show-notify", {
      notifyType: "info",
      title: "Sign upload required",
      details: "Please sign the upload request in your mobile wallet.",
    });

    let vaultKeySignature = await walletStore.getVaultKeySignature();
    emit("hide-notify");

    // Create upload entry in the store (but keep it pending until payment)
    const uploadId = uploadsStore.createUpload(files);
    currentUploadId.value = uploadId;

    // Initialize and show modal, then start the process
    initializeUploadSteps();
    pendingUploadFiles.value = {files, vaultKeySignature};
    showUploadModal.value = true;

    // Start the upload process immediately - this will trigger events that we'll handle
    // The modal will show progress through all steps including payment
    console.log(">>> FILEVIEWER STARTING UPLOAD PROCESS");

    // Set a timeout for the quote phase
    const quoteTimeout = setTimeout(() => {
      if (currentUploadStep.value === 'quoting' && !quoteData.value?.rawQuoteData) {
        console.error(">>> Quote timeout - no payment order received after 30 seconds");
        uploadError.value = "Quote request timed out. Please check your network connection and try again.";
        updateStepStatus('quoting', 'error', 'Quote request timed out');
        if (currentUploadId.value) {
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'failed',
            error: 'Quote request timed out',
            completedAt: new Date()
          });
        }
      }
    }, 30000); // 30 seconds timeout

    // Start the upload process
    await invoke("upload_files", {
      files,
      vaultKeySignature,
    });

    // Clear timeout if we got this far
    clearTimeout(quoteTimeout);

  } catch (error: any) {
    emit("hide-notify");
    console.error("Error in uploadFiles:", error);
    
    // Update modal if showing
    if (showUploadModal.value) {
      uploadError.value = error.message || "Unknown error occurred";
      updateStepStatus(currentUploadStep.value || 'processing', 'error', error.message);
    }

    if (currentUploadId.value) {
      uploadsStore.updateUpload(currentUploadId.value, {
        status: 'failed',
        error: error.message,
        completedAt: new Date()
      });
    }
    toast.add({
      severity: "error",
      summary: "Error starting upload",
      detail: error.message,
      life: 3000,
    });
  }
};

const handleGoBack = (target: any) => {
  breadcrumbs.value.pop();
  fileStore.changeDirectory(target);
};

const handleChangeDirectory = (target: any) => {
  if (!target?.children) {
    return;
  } else {
    breadcrumbs.value.push(target);
    fileStore.changeDirectory(target);
  }
};

const handleClickBreadcrumb = (crumb: any) => {
  const index = breadcrumbs.value.findIndex(breadcrumb => breadcrumb === crumb);
  breadcrumbs.value = breadcrumbs.value.slice(0, index + 1);
  fileStore.changeDirectory(crumb);
};

const handleDownloadFile = async (fileToDownload?: any) => {
  try {
    const file = fileToDownload || selectedFileItem.value;
    const fileName = file.name || 'downloaded_file';
    
    const downloadId = downloadsStore.createDownload(file);

    let fileData = file;
    if (!file.is_loaded && !file.is_loading) {
      downloadsStore.updateDownload(downloadId, { status: 'loading' });
      
      try {
        fileData = await fileStore.loadSingleFileData(file);
      } catch (error) {
        downloadsStore.updateDownload(downloadId, {
          status: 'failed',
          error: 'Could not load file data',
          completedAt: new Date()
        });
        toast.add({
          severity: 'error',
          summary: 'Download Failed',
          detail: 'Could not load file data for download.',
          life: 3000,
        });
        return;
      }
    }

    downloadsStore.updateDownload(downloadId, { status: 'downloading' });

    try {
      const downloadsPath = await downloadDir();
      const uniquePath = await invoke('get_unique_download_path', {
        downloadsPath,
        filename: fileName
      }) as string;

      if (fileData.file_access.Private) {
        await invoke('download_private_file', {
          dataMap: fileData.file_access.Private,
          toDest: uniquePath,
        });
      } else if (fileData.file_access.Public) {
        await invoke('download_public_file', {
          addr: fileData.file_access.Public,
          toDest: uniquePath,
        });
      }

      downloadsStore.updateDownload(downloadId, {
        status: 'completed',
        downloadPath: uniquePath,
        progress: 100,
        completedAt: new Date()
      });

      const finalFileName = uniquePath.split('/').pop() || fileName;
      toast.add({
        severity: 'success',
        summary: 'Download Complete',
        detail: `File saved as: ${finalFileName}`,
        life: 4000,
      });
    } catch (error: any) {
      downloadsStore.updateDownload(downloadId, {
        status: 'failed',
        error: error.message || 'Download failed',
        completedAt: new Date()
      });
      toast.add({
        severity: 'error',
        summary: 'Download Failed',
        detail: 'Failed to download the file.',
        life: 3000,
      });
    }
  } catch (error: any) {
    console.log('>>> Error in FileViewer.vue >> handleDownloadFile: ', error);
  }
};

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

const secondsToDate = (seconds: number): Date => {
  return new Date(seconds * 1000);
};

// Set up event listeners early, before component mount
const setupEventListeners = async () => {
  const {listen} = await import("@tauri-apps/api/event");

  // Listen for payment request events (simplified)
  await listen("payment-request", (event: any) => {
    console.log(">>> Received payment-request event:", event.payload);
    if (showUploadModal.value && event.payload) {
      const paymentData = event.payload;

      // Update quote data with payment information
      quoteData.value = {
        ...quoteData.value,
        totalCostNano: paymentData.total_cost_nano || "0",
        totalCostFormatted: paymentData.total_cost_formatted || "0 ATTO",
        paymentRequired: paymentData.payment_required || false,
        paymentOrderId: paymentData.order_id,
        rawQuoteData: {
          payments: paymentData.payments || []
        }
      };

      // DON'T update uploads store to active until payment is completed
      if (currentUploadId.value) {
        uploadsStore.updateUpload(currentUploadId.value, {
          status: 'payment'
        });
      }

      // Mark quoting as complete and show payment request
      updateStepStatus('quoting', 'completed', 'Quote received');
      updateStepStatus('payment-request', 'processing', 'Ready for payment');
      currentUploadStep.value = 'payment-request';

      console.log(">>> Payment request processed, total cost:", paymentData.total_cost_formatted);
    }
  });

  // Set up upload progress event listener
  await listen("upload-progress", (event: any) => {
    const payload = event.payload;

    switch (payload.type) {
      case "Started":
        uploadStore.startUpload(payload.total_files || 0, payload.total_size || 0);
        if (currentUploadId.value) {
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'processing',
            totalSize: payload.total_size || 0
          });
        }
        if (showUploadModal.value) {
          updateStepStatus('processing', 'processing', `Processing ${payload.total_files} file(s)...`);

          // Set quote data with real file info
          quoteData.value = {
            totalFiles: payload.total_files,
            totalSize: formatBytes(payload.total_size || 0)
          };
        }
        break;

      case "Processing":
        uploadStore.updateProcessing(
            payload.current_file || "",
            payload.files_processed || 0,
            payload.bytes_processed || 0
        );
        if (currentUploadId.value) {
          const progress = payload.total_bytes > 0 ? Math.round((payload.bytes_processed / payload.total_bytes) * 100) : 0;
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'processing',
            currentFile: payload.current_file,
            filesProcessed: payload.files_processed || 0,
            bytesProcessed: payload.bytes_processed || 0,
            progress
          });
        }
        if (showUploadModal.value) {
          const progress = payload.total_bytes > 0 ? Math.round((payload.bytes_processed / payload.total_bytes) * 100) : 0;
          updateStepStatus('processing', 'processing', `Processing: ${payload.current_file}`, progress);
        }
        break;

      case "Encrypting":
        uploadStore.updateEncrypting(payload.current_file || "");
        if (currentUploadId.value) {
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'encrypting',
            currentFile: payload.current_file
          });
        }
        if (showUploadModal.value) {
          updateStepStatus('processing', 'completed', 'Files processed');
          updateStepStatus('encrypting', 'processing', `Encrypting: ${payload.current_file}`);
        }
        break;

      case "RequestingPayment":
        uploadStore.updateRequestingPayment();
        if (currentUploadId.value) {
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'quoting'
          });
        }
        if (showUploadModal.value) {
          updateStepStatus('encrypting', 'completed', 'Files encrypted');
          updateStepStatus('quoting', 'processing', 'Getting storage quote...');
          // Note: Payment request will be shown when we get the payment-order event
        }
        break;

      case "Uploading":
        uploadStore.updateUploading(payload.chunks_uploaded || 0, payload.total_chunks || 0);
        if (currentUploadId.value) {
          const progress = payload.total_chunks > 0 ? Math.round((payload.chunks_uploaded / payload.total_chunks) * 100) : 0;
          // NOW the upload becomes active after payment completion
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'uploading',
            chunksUploaded: payload.chunks_uploaded || 0,
            totalChunks: payload.total_chunks || 0,
            progress
          });
        }
        if (showUploadModal.value) {
          // If we reach uploading, the payment was approved
          updateStepStatus('quoting', 'completed', 'Quote received');
          updateStepStatus('payment-request', 'completed', 'Payment authorized');

          // Close modal and show progress table
          showUploadModal.value = false;
          handleCancelUploadModal(); // Clean up modal state
        }
        break;

      case "Completed":
        uploadStore.completeUpload();
        if (currentUploadId.value) {
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'completed',
            progress: 100,
            completedAt: new Date()
          });
        }
        // Auto-refresh files after upload completion
        setTimeout(() => {
          fileStore.getAllFiles();
          uploadStore.resetUpload();
        }, 2000);
        break;

      case "Failed":
        uploadStore.failUpload(payload.error || "Unknown error");
        if (currentUploadId.value) {
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'failed',
            error: payload.error || "Unknown error",
            completedAt: new Date()
          });
        }
        if (showUploadModal.value) {
          uploadError.value = payload.error || "Unknown error";
        }
        setTimeout(() => {
          uploadStore.resetUpload();
        }, 5000);
        break;
    }
  });
};

// Call setupEventListeners immediately
setupEventListeners().catch(err => {
  console.error('>>> Error setting up event listeners:', err);
});

onMounted(async () => {
  try {
    fileStore.getAllFiles();
  } catch (err) {
    console.log('>>> Error getting files: ', err);
  }
});
</script>

<template>
  <div class="pr-[66px] pl-[66px] lg:pl-[110px] mt-10">
    <!-- Tab System -->
    <div class="-ml-[30px] -mr-[30px] lg:-ml-0 lg:-mr-0">
      <TabView v-model:activeIndex="activeTab" class="vault-tabs">
        <!-- Files Tab -->
        <TabPanel header="Files" :value="0">
          <!-- Upload Controls -->
          <div class="flex items-center justify-between mb-6">
            <div class="flex gap-3">
              <CommonButton
                  variant="secondary"
                  @click="openPickerAndUploadFiles"
                  :disabled="isUploading"
                  size="medium"
                  class="px-4 py-3 h-12"
              >
                <i class="pi pi-upload mr-2"/>
                <span v-if="!isUploading">Upload Files</span>
                <span v-else>Uploading...</span>
              </CommonButton>
              <CommonButton
                  variant="secondary"
                  @click="openFolderPickerAndUploadFiles"
                  :disabled="isUploading"
                  size="medium"
                  class="px-4 py-3 h-12"
              >
                <i class="pi pi-folder mr-2"/>
                <span v-if="!isUploading">Upload Folder</span>
                <span v-else>Uploading...</span>
              </CommonButton>
            </div>

            <div class="flex items-center gap-3">
              <div
                  v-if="currentDirectory?.parent"
                  class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
                  @click="handleGoBack(currentDirectory.parent)"
              >
                <i class="pi pi-reply -scale-x-100 translate"/>
              </div>

              <div
                  class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300 dark:bg-white dark:text-autonomi-blue-600 dark:hover:bg-white/70"
                  v-tooltip.bottom="'Refresh files'"
                  @click="fileStore.getAllFiles()"
              >
                <i class="pi pi-refresh"/>
              </div>

              <div
                  class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300 dark:bg-white dark:text-autonomi-blue-600 dark:hover:bg-white/70"
                  @click="$event => { refFilesViewMenu.toggle($event); }"
              >
                <i class="pi pi-bars"/>
              </div>
            </div>
          </div>

          <!-- Breadcrumbs -->
          <div
              v-if="breadcrumbs?.length > 0"
              class="flex gap-4 items-center text-sm font-semibold flex-wrap mb-4"
          >
            <div
                class="cursor-pointer transition-all duration-300 text-autonomi-text-secondary dark:text-autonomi-text-primary-dark"
                @click="handleClickBreadcrumb(rootDirectory)"
            >
              Root
            </div>
            <i class="text-xs pi pi-arrow-right text-autonomi-text-primary/70"/>

            <template v-for="(crumb, index) in breadcrumbs" :key="index">
              <div
                  :class="`cursor-pointer transition-all duration-300 ${
                  index === breadcrumbs.length - 1
                    ? 'text-autonomi-text-secondary'
                    : 'text-autonomi-text-primary/70'
                }`"
                  @click="handleClickBreadcrumb(crumb)"
              >
                {{ crumb.name }}
              </div>
              <i
                  v-if="index !== breadcrumbs.length - 1"
                  class="text-xs pi pi-arrow-right text-autonomi-text-primary/70"
              />
            </template>
          </div>

          <!-- Files Table -->
          <div
              v-if="viewTypeVault === 'list'"
              class="mt-6 grid grid-cols-12 font-semibold mb-10"
          >
            <div
                class="col-span-11 md:col-span-9 xl:col-span-8 pl-[80px] lg:pl-[110px] text-autonomi-red-300"
            >
              Name
            </div>
            <div class="hidden xl:block xl:col-span-3 text-autonomi-red-300">
              Upload Date
            </div>
            <div class="col-span-1 text-autonomi-red-300">
              <i class="pi pi-user"/>
            </div>

            <!-- Spacer -->
            <div class="col-span-12 h-10"/>

            <!-- Files Rows -->
            <template v-if="combinedFiles.length">
              <div
                  v-for="file in combinedFiles"
                  :key="file.path || file.name"
                  class="grid grid-cols-subgrid col-span-12 h-11 items-center odd:bg-autonomi-gray-100 dark:odd:bg-[#5b5d87] dark:bg-[#444565] dark:text-autonomi-text-primary-dark"
                  @click="handleChangeDirectory(file)"
                  :class="{
                  'cursor-pointer': !file.path || file.is_failed_archive,
                  'opacity-75': file.is_loading,
                  'opacity-75 bg-red-100 dark:bg-red-900/20 hover:bg-red-200': file.load_error || file.is_failed_archive,
                  'hover:bg-white dark:hover:bg-[#8587c5]': !(file.load_error || file.is_failed_archive)
                }"
              >
                <!-- Folder/File Name -->
                <div
                    class="col-span-11 md:col-span-9 xl:col-span-8 pl-[80px] lg:pl-[110px] flex items-center"
                >
                  <template v-if="file.is_failed_archive">
                    <!-- This is a failed archive -->
                    <i class="pi pi-exclamation-triangle mr-4 text-red-500"/>
                    <i class="pi pi-folder-open mr-2 text-red-500"/>
                    <span class="text-ellipsis overflow-hidden text-red-600 dark:text-red-400">
                      {{ file.name }} <span class="text-xs">({{ file.is_private ? 'Private' : 'Public' }})</span>
                    </span>
                  </template>
                  <template v-else-if="file?.path">
                    <!-- This is the file -->
                    <i
                        v-if="/\\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(file.name)"
                        class="pi pi-image mr-4"
                    />
                    <i
                        v-else-if="/\\.(pdf)$/i.test(file.name)"
                        class="pi pi-file-pdf mr-4"
                    />
                    <i v-else-if="/\\.(zip)$/i.test(file.name)" class="pi pi-box mr-4"/>
                    <i v-else class="pi pi-file mr-4"/>

                    <span
                        class="text-ellipsis overflow-hidden cursor-pointer"
                        @click.stop="handleFileNameClick(file)">
                      {{ file.name }}
                    </span>
                    <!-- Loading indicators for files -->
                    <i v-if="file.is_loading" class="pi pi-spinner pi-spin ml-2 text-sm text-blue-500"/>
                    <i v-else-if="file.load_error" class="pi pi-exclamation-triangle ml-2 text-sm text-red-500"
                       v-tooltip.top="'Failed to load file data'"/>
                  </template>
                  <template v-else>
                    <!-- This is the folder -->
                    <i class="pi pi-folder mr-4"/><span
                      class="line-clamp-one text-ellipsis"
                  >{{ file.name }}</span>
                  </template>
                </div>

                <!-- Upload Date -->
                <div
                    class="hidden xl:block xl:col-span-3 text-autonomi-text-primary dark:text-autonomi-text-primary-dark"
                >
                  {{
                    file?.metadata?.uploaded && !file.is_failed_archive
                        ? secondsToDate(file.metadata.uploaded).toLocaleString()
                        : ''
                  }}
                </div>

                <!-- Menu -->
                <template v-if="file.path && !file.is_failed_archive">
                  <div class="col-span-1">
                    <i
                        class="pi pi-ellipsis-v cursor-pointer hover:text-autonomi-gray-600  dark:hover:text-white"
                        @click.stop="
                        $event => {
                          selectedFileItem = file;
                          refFilesMenu.toggle($event);
                        }
                      "
                    />
                  </div>
                </template>
                <template v-else>
                  <div class="col-span-1"></div>
                </template>
              </div>
            </template>
            <template v-else>
              <div class="col-span-12 p-8 text-center text-gray-500">
                <div v-if="pendingVaultStructure">
                  <i class="pi pi-spinner pi-spin mr-4"/>Loading vault...
                </div>
                <div v-else>No files found.</div>
              </div>
            </template>
          </div>

          <!-- Grid View -->
          <div
              v-else-if="viewTypeVault === 'grid'"
              class="mt-6"
          >
            <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
              <div v-if="!combinedFiles.length" class="col-span-full p-8 text-center text-gray-500">
                <div v-if="pendingVaultStructure">
                  <i class="pi pi-spinner pi-spin mr-4"/>Loading vault...
                </div>
                <div v-else>No files found.</div>
              </div>
              <template v-else>
                <div
                    v-for="file in combinedFiles"
                    :key="file.path || file.name"
                    class="aspect-square h-[200px] text-autonomi-text-primary hover:bg-white rounded-lg hover:text-autonomi-text-secondary dark:bg-[#444565] dark:hover:bg-black/40 dark:hover:text-autonomi-text-primary-dark transition-all duration-500 p-4 border cursor-pointer"
                    @click="handleChangeDirectory(file)"
                >
                  <div class="flex flex-col items-center justify-center w-full h-full">
                    <template v-if="file.path">
                      <!-- Menu -->
                      <div class="self-end">
                        <i
                            class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                            @click.stop="
                            $event => {
                              selectedFileItem = file;
                              refFilesMenu.toggle($event);
                            }
                          "
                        />
                      </div>
                    </template>
                    
                    <div class="flex flex-col items-center justify-center flex-1 gap-2">
                      <i v-if="file.is_failed_archive" class="pi pi-exclamation-triangle text-4xl text-red-500"/>
                      <i v-else-if="file.path" class="pi pi-file text-4xl"/>
                      <i v-else class="pi pi-folder text-4xl"/>
                      
                      <span 
                          class="text-center text-sm truncate w-full cursor-pointer"
                          @click.stop="file.path ? handleFileNameClick(file) : null"
                      >
                        {{ file.name }}
                      </span>
                    </div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </TabPanel>

        <!-- Uploads Tab -->
        <TabPanel :header="`Uploads (${uploadsStore.activeUploads.length})`" :value="1">
          <div class="space-y-4">
            <h3 class="text-lg font-semibold">Active Uploads</h3>
            <div v-if="uploadsStore.activeUploads.length > 0" class="space-y-3">
              <div
                  v-for="upload in uploadsStore.activeUploads"
                  :key="upload.id"
                  class="p-4 border rounded-lg"
              >
                <div class="flex justify-between items-center mb-2">
                  <span class="font-medium">{{ upload.name }}</span>
                  <span class="text-sm text-gray-500">{{ upload.status }}</span>
                </div>
                <div class="w-full bg-gray-200 rounded-full h-2">
                  <div
                      class="bg-blue-500 h-2 rounded-full transition-all duration-300"
                      :style="`width: ${upload.progress}%`"
                  />
                </div>
                <div class="text-xs text-gray-500 mt-1">
                  {{ upload.progress }}% - {{ upload.filesProcessed }} of {{ upload.totalFiles }} files
                </div>
              </div>
            </div>
            <div v-else class="text-center py-8 text-gray-500">
              No active uploads
            </div>
          </div>
        </TabPanel>

        <!-- Downloads Tab -->
        <TabPanel :header="`Downloads (${downloadsStore.activeDownloads.length})`" :value="2">
          <div class="space-y-4">
            <h3 class="text-lg font-semibold">Active Downloads</h3>
            <div v-if="downloadsStore.activeDownloads.length > 0" class="space-y-3">
              <div
                  v-for="download in downloadsStore.activeDownloads"
                  :key="download.id"
                  class="p-4 border rounded-lg"
              >
                <div class="flex justify-between items-center mb-2">
                  <span class="font-medium">{{ download.fileName }}</span>
                  <span class="text-sm text-gray-500">{{ download.status }}</span>
                </div>
                <div class="w-full bg-gray-200 rounded-full h-2">
                  <div
                      class="bg-green-500 h-2 rounded-full transition-all duration-300"
                      :style="`width: ${download.progress}%`"
                  />
                </div>
                <div class="text-xs text-gray-500 mt-1">
                  {{ download.progress }}%
                </div>
              </div>
            </div>
            <div v-else class="text-center py-8 text-gray-500">
              No active downloads
            </div>
          </div>
        </TabPanel>
      </TabView>
    </div>


    <!-- Upload Progress Modal -->
    <DialogInvoice
        :visible="showUploadModal"
        :current-step="currentUploadStep"
        :steps="uploadSteps"
        :quote-data="quoteData"
        :error="uploadError"
        @close-modal="handleCloseUploadModal"
        @cancel-upload="handleCancelUploadModal"
        @pay-upload="handlePayUpload"
        @show-notify="emit('show-notify', $event)"
        @hide-notify="emit('hide-notify')"
    />

    <!-- MENUS -->
    <!-- FILES MENU POPOVER -->
    <Popover ref="refFilesMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
                v-for="item in menuFiles"
                :key="item.label"
                class="flex items-center gap-2 py-3 px-5 rounded-border rounded-2xl"
                :class="{
                'hover:bg-autonomi-gray-100 cursor-pointer': !item.disabled,
                'opacity-50 cursor-not-allowed': item.disabled
              }"
                @click="!item.disabled && item.command && item.command()"
            >
              <i :class="item.icon"/>
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- FILES VIEW MENU POPOVER -->
    <Popover ref="refFilesViewMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
                v-for="item in menuFilesView"
                :key="item.label"
                class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
                @click="item.command"
            >
              <i :class="item.icon"/>
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- UPLOAD MENU POPOVER -->
    <Popover ref="refUploadMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
                v-for="item in menuUploads"
                :key="item.label"
                class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
                @click="item.command"
            >
              <i :class="item.icon"/>
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- DOWNLOAD MENU POPOVER -->
    <Popover ref="refDownloadMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
                v-for="item in menuDownloads"
                :key="item.label"
                class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
                @click="item.command"
            >
              <i :class="item.icon"/>
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- DRAWER -->
    <Drawer
        v-model:visible="isVisibleFileInfo"
        header="Drawer"
        position="right"
    >
      <template #header>
        <div class="flex items-center gap-3">
          <div
              class="w-10 h-10 bg-autonomi-gray-500 rounded-full flex items-center justify-center"
          >
            <i class="pi pi-file text-white"/>
          </div>
          <div class="text-lg font-semibold text-autonomi-blue-600">
            Details
          </div>
        </div>
      </template>
      <div class="p-5 flex-col flex text-sm font-semibold">
        <div class="py-3">
          <div>Name</div>
          <div class="text-autonomi-text-primary">
            {{ selectedFileItem?.name }}
          </div>
        </div>

        <div class="py-3">
          <div>Size</div>
          <div class="text-autonomi-text-primary">
            {{ selectedFileItem?.metadata?.size ? formatBytes(selectedFileItem.metadata.size) : '' }}
          </div>
        </div>

        <div class="py-3">
          <div>Modified</div>
          <div class="text-autonomi-text-primary">
            {{
              selectedFileItem?.metadata?.modified
                  ? secondsToDate(
                      selectedFileItem.metadata.modified,
                  ).toLocaleString()
                  : ''
            }}
          </div>
        </div>

        <div class="py-3">
          <div>Created</div>
          <div class="text-autonomi-text-primary">
            {{
              selectedFileItem?.metadata?.created
                  ? secondsToDate(
                      selectedFileItem.metadata.created,
                  ).toLocaleString()
                  : ''
            }}
          </div>
        </div>
      </div>
    </Drawer>
  </div>
</template>