<script lang="ts" setup>
import {useFileStore} from '~/stores/files';
import {useToast} from 'primevue/usetoast';
import {useUserStore} from '~/stores/user';
import {useUploadStore} from '~/stores/upload';
import {useWalletStore} from '~/stores/wallet';
import {invoke} from '@tauri-apps/api/core';
import {downloadDir} from '@tauri-apps/api/path';
import {open} from '@tauri-apps/plugin-dialog';
import {basename} from '@tauri-apps/api/path';

const toast = useToast();
const fileStore = useFileStore();
const uploadStore = useUploadStore();
const walletStore = useWalletStore();
const {
  pendingVaultStructure,
  currentDirectory,
  currentDirectoryFiles,
  rootDirectory,
  files,
  failedArchives,
} = storeToRefs(fileStore);
const { uploadProgress } = storeToRefs(uploadStore);
const userStore = useUserStore();
// const autonomi = useAutonomiStore();
const {query} = storeToRefs(userStore);
const view = ref<'vault'>('vault');
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
const filteredFiles = computed(() => {
  try {
    if (!currentDirectory.value?.children?.length) {
      return [];
    }

    return currentDirectory.value.children.filter((folder: any) => {
      if (query.value) {
        // TODO: Change "parents" folder name
        return (
            folder.name.toLowerCase().includes(query.value.toLowerCase()) &&
            folder.name !== 'parents'
        );
      }

      return folder.name !== 'parents';
    });
  } catch (error) {
    // TODO: Handle error
    return [];
  }
});

// Combine regular files and failed archives
const combinedFiles = computed(() => {
  const regularFiles = filteredFiles.value || [];

  // Convert failed archives to a format compatible with files
  const failedArchiveFiles = failedArchives.value.map(archive => ({
    name: archive.name,
    is_failed_archive: true,
    is_private: archive.is_private,
    // Add these properties to match the structure expected by the UI
    is_loaded: false,
    is_loading: false,
    load_error: true,
    // Add empty path to indicate it's not a regular file
    path: `failed-archive://${archive.name}`,
    // Add empty metadata to avoid errors
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

// Upload functions
const emit = defineEmits(["show-notify", "hide-notify"]);

const isUploading = computed(() => uploadStore.uploadProgress.isUploading);

const openPickerAndUploadFiles = async () => {
  // Open the file picker.
  let selected = await open({multiple: true});

  // User did not select any files in the dialog.
  if (selected === null) {
    return;
  }

  // User might have selected a single file, turn into one-element array.
  if (!Array.isArray(selected)) {
    selected = [selected];
  }

  // Turn into `File` objects, giving the file the name of the filename in the path.
  const files = await Promise.all(
      selected.map(async (file) => {
        return {path: file, name: await basename(file)};
      })
  );

  await uploadFiles(files);
};

const openFolderPickerAndUploadFiles = async () => {
  // Open the folder picker
  const selected = await open({
    directory: true,
  });

  // User did not select a folder in the dialog
  if (selected === null) {
    return;
  }

  const files = [{path: selected, name: await basename(selected)}];

  await uploadFiles(files);
};

// Initialize upload steps
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

// Common function to upload files
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

    // Initialize and show modal, then start the process
    initializeUploadSteps();
    pendingUploadFiles.value = { files, vaultKeySignature };
    showUploadModal.value = true;

    // Start the upload process immediately - this will trigger events that we'll handle
    // The modal will show progress through all steps including payment
    console.log(">>> FILEVIEWER STARTING UPLOAD PROCESS");
    await invoke("upload_files", {
      files,
      vaultKeySignature,
    });

  } catch (error: any) {
    emit("hide-notify");
    uploadError.value = error.message;
    toast.add({
      severity: "error",
      summary: "Error starting upload",
      detail: error.message,
      life: 3000,
    });
  }
};

// Payment is handled automatically by the wallet/backend
// No separate confirmation needed - the upload flow continues automatically

const handleCancelUploadModal = () => {
  showUploadModal.value = false;
  pendingUploadFiles.value = null;
  uploadSteps.value = [];
  currentUploadStep.value = '';
  uploadError.value = '';
  quoteData.value = null;
};

const handleCloseUploadModal = () => {
  const hasActiveProcessing = uploadSteps.value.some(step => step.status === 'processing');
  if (!hasActiveProcessing) {
    handleCancelUploadModal();
  }
};

const handleGoBack = (target: any) => {
  // Update breadcrumbs
  breadcrumbs.value.pop();

  // This is a folder, change directory
  fileStore.changeDirectory(target);
};

const handleChangeDirectory = (target: any) => {
  if (!target?.children) {
    // This is a file
    // toast.add({
    //   severity: "info",
    //   summary: "File",
    //   detail: "TODO: Handle Click on File",
    //   life: 6000,
    // });
    return;
  } else {
    // Update breadcrumbs
    breadcrumbs.value.push(target);
    // This is a folder, change directory
    fileStore.changeDirectory(target);
  }
};

const handleClickBreadcrumb = (crumb: any) => {
  // Remove all breadcrumbs after the clicked one
  const index = breadcrumbs.value.findIndex(breadcrumb => breadcrumb === crumb);

  breadcrumbs.value = breadcrumbs.value.slice(0, index + 1);

  fileStore.changeDirectory(crumb);
};

const handleStartUpload = () => {
  toast.add({
    severity: 'info',
    summary: 'Upload',
    detail: 'TODO: Handle Start Upload',
    life: 6000,
  });
};

const handlePauseUpload = () => {
  toast.add({
    severity: 'info',
    summary: 'Upload',
    detail: 'TODO: Handle Pause Upload',
    life: 6000,
  });
};

const handleCancelUpload = () => {
  toast.add({
    severity: 'info',
    summary: 'Upload',
    detail: 'TODO: Handle Cancel Upload',
    life: 6000,
  });
};

const handleStartDownload = () => {
  toast.add({
    severity: 'info',
    summary: 'Download',
    detail: 'TODO: Handle Start Download',
    life: 6000,
  });
};

const handlePauseDownload = () => {
  toast.add({
    severity: 'info',
    summary: 'Download',
    detail: 'TODO: Handle Pause Download',
    life: 6000,
  });
};

const handleCancelDownload = () => {
  toast.add({
    severity: 'info',
    summary: 'Download',
    detail: 'TODO: Handle Cancel Download',
    life: 6000,
  });
};

const menuUploads = ref([
  {
    label: 'Start',
    icon: 'pi pi-check',
    command: handleStartUpload,
  },
  {
    label: 'Pause',
    icon: 'pi pi-pause',
    command: handlePauseUpload,
  },
  {
    label: 'Cancel',
    icon: 'pi pi-times',
    command: handleCancelUpload,
  },
]);

const menuDownloads = ref([
  {
    label: 'Start',
    icon: 'pi pi-check',
    command: handleStartDownload,
  },
  {
    label: 'Pause',
    icon: 'pi pi-pause',
    command: handlePauseDownload,
  },
  {
    label: 'Cancel',
    icon: 'pi pi-times',
    command: handleCancelDownload,
  },
]);

const handleToggleUploadMenu = (event: any) => {
  refUploadMenu.value.toggle(event);
};

const handleToggleDownloadMenu = (event: any) => {
  refUploadMenu.value.toggle(event);
};

const handleToggleFileMenu = (event: any) => {
  refFilesMenu.value.toggle(event);
};

const handleRenameFile = () => {
  toast.add({
    severity: 'info',
    summary: 'File',
    detail: 'TODO: Handle Rename File',
    life: 6000,
  });
};

const handleMoveFile = () => {
  toast.add({
    severity: 'info',
    summary: 'File',
    detail: 'TODO: Handle Move File',
    life: 6000,
  });
};

const handleDownloadFile = async () => {
  try {
    const file = selectedFileItem.value;

    // Load file data on-demand if not already loaded
    let fileData = file;
    if (!file.is_loaded && !file.is_loading) {
      try {
        fileData = await fileStore.loadSingleFileData(file);
      } catch (error) {
        toast.add({
          severity: 'error',
          summary: 'Download Failed',
          detail: 'Could not load file data for download.',
          life: 3000,
        });
        return;
      }
    } else if (file.is_loading) {
      toast.add({
        severity: 'info',
        summary: 'Download in Progress',
        detail: 'File is already being prepared for download...',
        life: 3000,
      });
      return;
    }

    toast.add({
      severity: 'info',
      summary: 'File Download',
      detail:
          'Downloading file...check your downloads folder (this may take some time)',
      life: 6000,
    });

    const downloadsPath = await downloadDir();
    // Extract filename from path - use the original file.name from UI or extract from path
    const fileName = file.name || fileData.path.split('/').pop() || 'downloaded_file';

    try {
      if (fileData.file_access.Private) {
        // Private file download
        const downloadPrivateFileArgs = {
          dataMap: fileData.file_access.Private,
          toDest: `${downloadsPath}/${fileName}`,
        };
        await invoke('download_private_file', downloadPrivateFileArgs);
      } else if (fileData.file_access.Public) {
        // Public file download
        const downloadPublicFileArgs = {
          addr: fileData.file_access.Public,
          toDest: `${downloadsPath}/${fileName}`,
        };
        await invoke('download_public_file', downloadPublicFileArgs);
      }

      // Success - file has been downloaded to the downloads folder
      toast.add({
        severity: 'success',
        summary: 'Download Complete',
        detail: `File saved to downloads folder: ${fileName}`,
        life: 4000,
      });
    } catch (error) {
      console.error('Error downloading file:', error);
      toast.add({
        severity: 'error',
        summary: 'Download Failed',
        detail: 'Failed to download the file.',
        life: 3000,
      });
      return;
    }
  } catch (error) {
    console.log('>>> Error in FileViewer.vue >> handleDownloadFile: ', error);
  }
};

const handleDeleteFile = () => {
  toast.add({
    severity: 'info',
    summary: 'File',
    detail: 'TODO: Handle Delete File',
    life: 6000,
  });
};

const handleInfoFile = () => {
  isVisibleFileInfo.value = true;
};

const handleFileNameClick = (file: any) => {
  // Only open info for files, not folders
  if (file.path) {
    selectedFileItem.value = file;
    handleInfoFile();
  }
};

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
      command: handleDownloadFile,
    });
  } else {
    // Always show download button - we'll load data on-demand
    items.push({
      label: 'Download',
      icon: 'pi pi-download',
      command: handleDownloadFile,
    });
  }

  items.push({
    label: 'Info',
    icon: 'pi pi-info-circle',
    command: handleInfoFile,
  });

  return items;
});

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

const handleToggleFilesViewMenu = (event: any) => {
  refFilesViewMenu.value.toggle(event);
};

const secondsToDate = (seconds: number): Date => {
  return new Date(seconds * 1000);
};

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const sizes = ["Bytes", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
};

onMounted(async () => {
  try {
    // TODO: Check user / wallet permissions and details
    fileStore.getAllFiles();

    console.log('>>> Local Drive: ', rootDirectory);
    console.log('>>> Current directory: ', currentDirectory);
    console.log('>>> Current directory files: ', currentDirectoryFiles);

    console.log('>>> Filtered files: ', filteredFiles);

    // Set up upload progress event listener
    const { listen } = await import("@tauri-apps/api/event");
    await listen("upload-progress", (event: any) => {
      const payload = event.payload;
      
      switch (payload.type) {
        case "Started":
          uploadStore.startUpload(payload.total_files || 0, payload.total_size || 0);
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
          if (showUploadModal.value) {
            const progress = payload.total_bytes > 0 ? Math.round((payload.bytes_processed / payload.total_bytes) * 100) : 0;
            updateStepStatus('processing', 'processing', `Processing: ${payload.current_file}`, progress);
          }
          break;
          
        case "Encrypting":
          uploadStore.updateEncrypting(payload.current_file || "");
          if (showUploadModal.value) {
            updateStepStatus('processing', 'completed', 'Files processed');
            updateStepStatus('encrypting', 'processing', `Encrypting: ${payload.current_file}`);
          }
          break;
          
        case "RequestingPayment":
          uploadStore.updateRequestingPayment();
          if (showUploadModal.value) {
            updateStepStatus('encrypting', 'completed', 'Files encrypted');
            updateStepStatus('quoting', 'processing', 'Getting storage quote...');
            // Note: Payment request will be shown when we get the payment-order event
          }
          break;
          
        case "Uploading":
          uploadStore.updateUploading(payload.chunks_uploaded || 0, payload.total_chunks || 0);
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
          // Auto-refresh files after upload completion
          setTimeout(() => {
            fileStore.getAllFiles();
            uploadStore.resetUpload();
          }, 2000);
          break;
          
        case "Failed":
          uploadStore.failUpload(payload.error || "Unknown error");
          if (showUploadModal.value) {
            uploadError.value = payload.error || "Unknown error";
          }
          setTimeout(() => {
            uploadStore.resetUpload();
          }, 5000);
          break;
      }
    });

    // Listen for payment order events (these contain quote data - this IS the payment request)
    await listen("payment-order", (event: any) => {
      if (showUploadModal.value && event.payload) {
        const orderData = event.payload;
        
        // Update quote data with real cost information
        if (quoteData.value) {
          quoteData.value.totalCostNano = orderData.totalCost || "0";
          quoteData.value.costPerFileNano = orderData.costPerFile || "0";
          quoteData.value.rawQuoteData = orderData;
        }
        
        // Mark quoting as complete and show payment request (which is the same as showing the quote/cost)
        updateStepStatus('quoting', 'completed', 'Quote received');
        updateStepStatus('payment-request', 'processing', 'Awaiting wallet authorization...');
        currentUploadStep.value = 'payment-request';
        
        // At this point the quote data is shown as the payment request
        // The wallet will handle the authorization automatically
      }
    });
  } catch (err) {
    // TODO: Handle error
    console.log('>>> Error getting files: ', err);
  }
});
</script>

<template>
  <div class="pr-[66px] pl-[66px] lg:pl-[110px] mt-10">
    <!-- View Toggler -->
    <div
        v-if="view === 'vault'"
        class="flex items-center justify-between -mr-[30px] lg:-mr-0"
    >
      <!-- Upload Buttons -->
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
      
      <!-- Navigation and Controls -->
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

        <!-- Individual File Loading Indicator -->
        <div
            v-if="loadingProgress.loading > 0"
            class="flex items-center gap-2 px-3 py-2 bg-blue-100 dark:bg-blue-900/30 rounded-full text-sm font-medium"
        >
          <i class="pi pi-spinner pi-spin text-blue-500"/>
          <span class="text-blue-700 dark:text-blue-300">
            Loading {{ loadingProgress.loading }} file{{ loadingProgress.loading > 1 ? 's' : '' }}...
          </span>
          <span v-if="loadingProgress.errors > 0" class="text-red-500">
            ({{ loadingProgress.errors }} errors)
          </span>
        </div>

        <div
            class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300 dark:bg-white dark:text-autonomi-blue-600 dark:hover:bg-white/70"
            @click="
            $event => {
              handleToggleFilesViewMenu($event);
            }
          "
        >
          <i class="pi pi-bars"/>
        </div>
      </div>
    </div>

    <!-- Breadcrumbs -->
    <div
        class="flex gap-4 items-center text-sm font-semibold flex-wrap mt-4 -ml-[30px] lg:-ml-0"
        v-if="breadcrumbs?.length > 0 && view === 'vault'"
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

    <!-- Upload Progress Table -->
    <div 
      v-if="uploadProgress.isUploading || uploadProgress.error" 
      class="mt-8 -mr-[30px] -ml-[30px] lg:-ml-0 lg:-mr-0"
    >
      <div class="bg-white dark:bg-autonomi-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-autonomi-gray-700 overflow-hidden">
        <!-- Table Header -->
        <div class="bg-gray-50 dark:bg-autonomi-gray-900 px-6 py-3 border-b border-gray-200 dark:border-autonomi-gray-700">
          <h3 class="text-sm font-semibold text-gray-900 dark:text-autonomi-text-primary-dark flex items-center gap-2">
            <i :class="uploadProgress.error ? 'pi pi-exclamation-triangle text-red-500' : 'pi pi-upload text-blue-500'"/>
            {{ uploadProgress.error ? 'Upload Failed' : 'Upload in Progress' }}
          </h3>
        </div>
        
        <!-- Table Content -->
        <div class="p-6">
          <div class="grid grid-cols-12 gap-4 items-center">
            <!-- File Name -->
            <div class="col-span-12 md:col-span-4 lg:col-span-3">
              <div class="text-sm font-medium text-gray-900 dark:text-autonomi-text-primary-dark">
                {{ uploadProgress.currentFile || `${uploadProgress.totalFiles} file(s)` }}
              </div>
              <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {{ uploadProgress.filesProcessed }} of {{ uploadProgress.totalFiles }} files
              </div>
            </div>
            
            <!-- Progress Bar -->
            <div class="col-span-12 md:col-span-5 lg:col-span-6">
              <div v-if="!uploadProgress.error" class="space-y-2">
                <div class="flex justify-between text-xs text-gray-600 dark:text-gray-300">
                  <span>{{ uploadProgress.statusMessage }}</span>
                  <span>{{ uploadProgress.progressPercentage }}%</span>
                </div>
                <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                  <div 
                    class="bg-blue-500 h-2 rounded-full transition-all duration-300 ease-out"
                    :style="`width: ${uploadProgress.progressPercentage}%`"
                  ></div>
                </div>
              </div>
              <div v-else class="text-sm text-red-600 dark:text-red-400">
                {{ uploadProgress.statusMessage }}
              </div>
            </div>
            
            <!-- Size Info -->
            <div class="col-span-12 md:col-span-3 lg:col-span-3 text-right">
              <div v-if="uploadProgress.totalBytes > 0" class="text-sm text-gray-600 dark:text-gray-300">
                {{ formatBytes(uploadProgress.bytesProcessed) }}
              </div>
              <div v-if="uploadProgress.totalBytes > 0" class="text-xs text-gray-500 dark:text-gray-400">
                of {{ formatBytes(uploadProgress.totalBytes) }}
              </div>
              
              <!-- Upload chunks info for upload phase -->
              <div v-if="uploadProgress.totalChunks > 0 && !uploadProgress.error" class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {{ uploadProgress.chunksUploaded }} / {{ uploadProgress.totalChunks }} chunks
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- View Navigation -->
    <div class="mt-[62px] -ml-[30px] -mr-[30px] lg:-ml-0 lg:-mr-0">
      <div class="grid grid-cols-12 h-10">
        <div
            class="col-span-12 lg:col-span-12 xl:col-span-12 flex flex-col justify-between"
        >
          <div
              :class="`text-sm font-semibold cursor-pointer transition-all duration-300 ${
              view === 'vault'
                ? 'text-autonomi-text-secondary dark:text-autonomi-text-primary-dark'
                : 'text-autonomi-text-primary/70'
            }`"
              @click="view = 'vault'"
          >
            Files
          </div>
          <div
              :class="`h-1 transition-all duration-300 ${
              view === 'vault' ? 'bg-autonomi-blue-600 dark:bg-autonomi-blue-200' : 'bg-autonomi-blue-200'
            }`"
          />
        </div>
        <!--        <div-->
        <!--          class="col-span-4 lg:col-span-3 xl:col-span-2 flex flex-col justify-between"-->
        <!--        >-->
        <!--          <div-->
        <!--            :class="`text-sm font-semibold pl-3 lg:pl-12 cursor-pointer transition-all duration-300 ${-->
        <!--              view === 'uploads'-->
        <!--                ? 'text-autonomi-text-secondary'-->
        <!--                : 'text-autonomi-text-primary/70'-->
        <!--            }`"-->
        <!--            @click="view = 'uploads'"-->
        <!--          >-->
        <!--            Uploads-->
        <!--          </div>-->
        <!--          <div-->
        <!--            :class="`h-1 transition-all duration-300 ${-->
        <!--              view === 'uploads'-->
        <!--                ? 'bg-autonomi-blue-600'-->
        <!--                : 'bg-autonomi-blue-200'-->
        <!--            }`"-->
        <!--          />-->
        <!--        </div>-->
        <!--        <div-->
        <!--          class="col-span-4 lg:col-span-3 xl:col-span-2 flex flex-col justify-between"-->
        <!--        >-->
        <!--          <div-->
        <!--            :class="`text-sm font-semibold pl-2 lg:pl-12 cursor-pointer transition-all duration-300 ${-->
        <!--              view === 'downloads'-->
        <!--                ? 'text-autonomi-text-secondary'-->
        <!--                : 'text-autonomi-text-primary/70'-->
        <!--            }`"-->
        <!--            @click="view = 'downloads'"-->
        <!--          >-->
        <!--            Downloads-->
        <!--          </div>-->
        <!--          <div-->
        <!--            :class="`h-1 transition-all duration-300 ${-->
        <!--              view === 'downloads'-->
        <!--                ? 'bg-autonomi-blue-600'-->
        <!--                : 'bg-autonomi-blue-200'-->
        <!--            }`"-->
        <!--          />-->
        <!--        </div>-->
        <!--        <div class="col-span-6 hidden xl:flex flex-col justify-between">-->
        <!--          <div class="flex gap-7 self-end">-->
        <!--            <div class="flex items-center gap-2">-->
        <!--              <div class="h-2 w-2 rounded-full bg-autonomi-red-300" />-->
        <!--              <div class="text-sm font-semibold text-autonomi-text-secondary">-->
        <!--                Completed-->
        <!--              </div>-->
        <!--            </div>-->

        <!--            <div class="flex items-center gap-2">-->
        <!--              <div class="h-2 w-2 rounded-full bg-autonomi-blue-600" />-->
        <!--              <div class="text-sm font-semibold text-autonomi-text-secondary">-->
        <!--                Paused-->
        <!--              </div>-->
        <!--            </div>-->

        <!--            <div class="flex items-center gap-2">-->
        <!--              <div class="h-2 w-2 rounded-full bg-autonomi-gray-300" />-->
        <!--              <div class="text-sm font-semibold text-autonomi-text-secondary">-->
        <!--                Not Started-->
        <!--              </div>-->
        <!--            </div>-->
        <!--          </div>-->
        <!--          <div class="h-1 bg-autonomi-blue-200" />-->
        <!--        </div>-->
        <!--        <div-->
        <!--          class="hidden col-span-3 lg:flex flex-col justify-between xl:hidden"-->
        <!--        >-->
        <!--          <div />-->
        <!--          <div class="h-1 bg-autonomi-blue-200" />-->
        <!--        </div>-->
      </div>

      <!--      &lt;!&ndash; MOBILE LEGEND &ndash;&gt;-->
      <!--      <div class="flex gap-7 justify-end xl:hidden mt-6">-->
      <!--        <div class="flex items-center gap-2">-->
      <!--          <div class="h-2 w-2 rounded-full bg-autonomi-red-300" />-->
      <!--          <div class="text-sm font-semibold text-autonomi-text-secondary">-->
      <!--            Completed-->
      <!--          </div>-->
      <!--        </div>-->

      <!--        <div class="flex items-center gap-2">-->
      <!--          <div class="h-2 w-2 rounded-full bg-autonomi-blue-600" />-->
      <!--          <div class="text-sm font-semibold text-autonomi-text-secondary">-->
      <!--            Paused-->
      <!--          </div>-->
      <!--        </div>-->

      <!--        <div class="flex items-center gap-2">-->
      <!--          <div class="h-2 w-2 rounded-full bg-autonomi-gray-300" />-->
      <!--          <div class="text-sm font-semibold text-autonomi-text-secondary">-->
      <!--            Not Started-->
      <!--          </div>-->
      <!--        </div>-->
      <!--      </div>-->
    </div>

    <!-- Files Views -->
    <div class="mt-11 -mr-[66px] -ml-[110px]">
      <!-- Viewing: Vault (LIST) -->
      <div
          v-if="view === 'vault' && viewTypeVault === 'list'"
          class="grid grid-cols-12 font-semibold mb-10"
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

        <!-- Vault Files Rows -->
        <template v-if="combinedFiles.length">
          <div
              v-for="file in combinedFiles"
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
                    v-if="/\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(file.name)"
                    class="pi pi-image mr-4"
                />
                <i
                    v-if="/\.(pdf)$/i.test(file.name)"
                    class="pi pi-file-pdf mr-4"
                />
                <i v-if="/\.(zip)$/i.test(file.name)" class="pi pi-box mr-4"/>

                <span 
                  class="text-ellipsis overflow-hidden cursor-pointer"
                  @click="handleFileNameClick(file)">{{
                    file.name
                  }}</span>
                <!-- Loading indicators for files -->
                <i v-if="file.is_loading" class="pi pi-spinner pi-spin ml-2 text-sm text-blue-500"/>
                <i v-else-if="file.load_error" class="pi pi-exclamation-triangle ml-2 text-sm text-red-500"
                   v-tooltip.top="'Failed to load file data'"/>
              </template>
              <template v-else>
                <!-- This is the folder -->
                <i class="pi pi-folder mr-4"/><span
                  class="line-clamp-one text-ellipsis"
              >{{ file.name }}</span
              >
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
                    @click="
                    $event => {
                      // TODO: Update key:values to match api
                      selectedFileItem = file;
                      handleToggleFileMenu($event);
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
          <!-- No files or archives found -->
          <div
              class="grid grid-cols-subgrid col-span-12 items-center justify-center min-h-[100px] font-semibold text-4xl text-autonomi-blue-600/50 dark:text-autonomi-text-primary-dark"
          >
            <div v-if="pendingVaultStructure" class="col-span-12 pl-[150px]">
              <i class="pi pi-spinner pi-spin mr-4"/>Loading vault...
            </div>
            <div v-else class="col-span-12 pl-[150px]">No files found.</div>
          </div>
        </template>
      </div>

      <!-- Viewing: Vault (GRID) -->
      <div
          v-if="view === 'vault' && viewTypeVault === 'grid'"
          class="grid grid-cols-12 font-semibold mb-10"
      >
        <div class="col-span-12 pl-[80px] lg:pl-[110px] text-autonomi-red-300">
          Name
        </div>

        <!-- Spacer -->
        <div class="col-span-12 h-10"/>

        <!-- Vault Files Rows -->
        <div
            class="col-span-12 grid grid-cols-12 ml-[80px] lg:ml-[110px] mr-[30px] lg:mr-[66px] gap-2"
        >
          <template v-if="filteredFiles.length">
            <div
                v-for="file in filteredFiles"
                class="col-span-6 md:col-span-4 xl:col-span-3 aspect-squarez h-[200px] text-autonomi-text-primary hover:bg-white rounded-lg hover:text-autonomi-text-secondary dark:bg-[#444565] dark:hover:bg-black/40 dark:hover:text-autonomi-text-primary-dark transition-all duration-500"
                :class="{
                'cursor-pointer': !file.path,
                'opacity-75': file.is_loading,
                'opacity-75 bg-red-100 dark:bg-red-900/20 hover:bg-red-200': file.load_error,
                'hover:bg-white dark:hover:bg-[#8587c5]': !file.load_error
              }"
                @click="handleChangeDirectory(file)"
            >
              <div
                  class="flex flex-col items-center justify-center w-full h-full p-2"
              >
                <!-- Menu -->
                <template v-if="file.path">
                  <div class="self-end">
                    <i
                        class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                        @click="
                        $event => {
                          // TODO: Update key:values to match api
                          selectedFileItem = file;
                          handleToggleFileMenu($event);
                        }
                      "
                    />
                  </div>
                </template>

                <!-- Folder/File Name -->
                <div
                    class="flex flex-col flex-1 items-center justify-center gap-3 w-full overflow-hidden"
                >
                  <template v-if="file?.path">
                    <!-- This is the file -->
                    <i
                        v-if="
                        /\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(file.name)
                      "
                        class="pi pi-image text-4xl"
                    />
                    <i
                        v-if="/\.(pdf)$/i.test(file.name)"
                        class="pi pi-file-pdf mr-4"
                    />
                    <i
                        v-if="/\.(zip)$/i.test(file.name)"
                        class="pi pi-box mr-4"
                    />

                    <div class="flex items-center justify-center gap-2 w-full">
                      <span v-tooltip.bottom="file.name"
                            class="text-ellipsis overflow-hidden line-clamp-1 block text-center cursor-pointer"
                            @click="handleFileNameClick(file)">{{
                          file.name
                        }}</span>
                      <i v-if="file.is_loading" class="pi pi-spinner pi-spin text-xs text-blue-500"/>
                      <i v-else-if="file.load_error" class="pi pi-exclamation-triangle text-xs text-red-500"
                         v-tooltip.top="'Failed to load file data'"/>
                    </div>
                  </template>
                  <template v-else>
                    <!-- This is the folder -->
                    <i class="pi pi-folder text-4xl lg:text-6xl"/>
                    <div class="text-ellipsis overflow-hidden">
                      {{ file.name }}
                    </div>
                  </template>
                </div>
              </div>
            </div>

            <!-- Failed Archives Section (Grid View) -->
            <template v-if="failedArchives.length > 0">
              <!-- Section Header -->
              <div class="col-span-12 mt-8 mb-4">
                <h3 class="text-lg font-semibold text-red-500 dark:text-red-400">Failed Archives</h3>
                <p class="text-sm text-gray-600 dark:text-gray-400">The following archives could not be loaded</p>
              </div>

              <!-- Failed Archives Grid Items -->
              <div
                  v-for="archive in failedArchives"
                  class="col-span-6 md:col-span-4 xl:col-span-3 aspect-squarez h-[200px] text-red-600 bg-red-50 rounded-lg dark:bg-red-900/20 dark:text-red-400 transition-all duration-500"
              >
                <div class="flex flex-col items-center justify-center w-full h-full p-2">
                  <!-- Archive Icon and Name -->
                  <div class="flex flex-col flex-1 items-center justify-center gap-3 w-full overflow-hidden">
                    <i class="pi pi-exclamation-triangle text-2xl text-red-500 mb-2"/>
                    <i class="pi pi-folder-open text-4xl text-red-500"/>
                    <div class="flex flex-col items-center">
                      <span v-tooltip.bottom="archive.name"
                            class="text-ellipsis overflow-hidden line-clamp-1 block text-center">
                        {{ archive.name }}
                      </span>
                      <span class="text-xs mt-1">({{ archive.is_private ? 'Private' : 'Public' }})</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>
          </template>
          <template v-else>
            <!-- Show failed archives in grid view if there are any, even if no regular files -->
            <template v-if="failedArchives.length > 0">
              <!-- Section Header -->
              <div class="col-span-12 mt-8 mb-4">
                <h3 class="text-lg font-semibold text-red-500 dark:text-red-400">Failed Archives</h3>
                <p class="text-sm text-gray-600 dark:text-gray-400">The following archives could not be loaded</p>
              </div>

              <!-- Failed Archives Grid Items -->
              <div
                  v-for="archive in failedArchives"
                  class="col-span-6 md:col-span-4 xl:col-span-3 aspect-squarez h-[200px] text-red-600 bg-red-50 rounded-lg dark:bg-red-900/20 dark:text-red-400 transition-all duration-500"
              >
                <div class="flex flex-col items-center justify-center w-full h-full p-2">
                  <!-- Archive Icon and Name -->
                  <div class="flex flex-col flex-1 items-center justify-center gap-3 w-full overflow-hidden">
                    <i class="pi pi-exclamation-triangle text-2xl text-red-500 mb-2"/>
                    <i class="pi pi-folder-open text-4xl text-red-500"/>
                    <div class="flex flex-col items-center">
                      <span v-tooltip.bottom="archive.name"
                            class="text-ellipsis overflow-hidden line-clamp-1 block text-center">
                        {{ archive.name }}
                      </span>
                      <span class="text-xs mt-1">({{ archive.is_private ? 'Private' : 'Public' }})</span>
                    </div>
                  </div>
                </div>
              </div>
            </template>
            <!-- No files or archives found -->
            <div v-else
                 class="grid grid-cols-subgrid col-span-12 items-center justify-center min-h-[100px] font-semibold text-4xl text-autonomi-blue-600/50"
            >
              <div v-if="pendingVaultStructure" class="col-span-12 pl-[150px]">
                <i class="pi pi-spinner pi-spin mr-4"/>Loading vault...
              </div>
              <div v-else class="col-span-12 pl-[150px]">No files found.</div>
            </div>
          </template>
        </div>
      </div>

      <!-- Viewing: Uploads -->
      <div
          v-if="view === 'uploads'"
          class="mr-[33px] lg:mr-[66px] ml-[80px] lg:ml-[110px] font-semibold mb-10 flex flex-col gap-4"
      >
        <template v-for="item in [1, 2, 3]">
          <div class="flex items-center gap-4">
            <!-- Row Details -->
            <div class="grid grid-cols-12 gap-y-2 flex-1">
              <div class="col-span-12 lg:col-span-3 flex items-center">
                <div>MyRandomFileName.zip</div>
                <div class="ml-auto lg:hidden">
                  <i
                      class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                      @click="
                      $event => {
                        selectedUploadItem = item;
                        handleToggleUploadMenu($event);
                      }
                    "
                  />
                </div>
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 20).toFixed(2) }} GB
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 12).toFixed(0) }} of 265
              </div>
              <div class="hidden lg:block col-span-3 whitespace-nowrap">
                <span class="text-autonomi-red-300"
                >{{ (Math.random() * 100).toFixed(2) }}%</span
                >
                complete
              </div>
              <div class="col-span-12 flex gap-[2px]">
                <div
                    v-for="item in Array(100)"
                    class="h-[22px] flex-1"
                    :class="`${
                    Math.random() > 0.5
                      ? 'bg-autonomi-red-300'
                      : Math.random() > 0.5
                      ? 'bg-autonomi-gray-500'
                      : 'bg-autonomi-blue-600'
                  }`"
                ></div>
              </div>
              <div
                  class="col-span-12 lg:hidden text-xs font-semibold text-autonomi-text-primary flex justify-between gap-2"
              >
                <div>14.2.GB</div>
                <div>€43.25</div>
                <div>12 of 265</div>
                <div>2h 56m</div>
              </div>
            </div>
            <!-- Row Menu -->
            <div class="self-end hidden lg:block">
              <i
                  class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                  @click="
                  $event => {
                    selectedUploadItem = item;
                    handleToggleUploadMenu($event);
                  }
                "
              />
            </div>
          </div>
        </template>
      </div>

      <!-- Viewing: Downloads -->
      <div
          v-if="view === 'downloads'"
          class="mr-[30px] lg:mr-[66px] ml-[80px] lg:ml-[110px] font-semibold mb-10 flex flex-col gap-4"
      >
        <template v-for="item in Array(5)">
          <div class="flex items-center gap-4">
            <!-- Row Details -->
            <div class="grid grid-cols-12 gap-y-2 flex-1">
              <div class="col-span-12 lg:col-span-6 flex items-center">
                <div>MyRandomFileName.zip</div>
                <div class="ml-auto lg:hidden">
                  <i
                      class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                      @click="
                      $event => {
                        selectedDownloadItem = item;
                        handleToggleDownloadMenu($event);
                      }
                    "
                  />
                </div>
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 20).toFixed(2) }} GB
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 12).toFixed(0) }} of 265
              </div>
              <div
                  class="hidden lg:block col-span-2 text-right whitespace-nowrap"
              >
                <span class="text-autonomi-red-300"
                >{{ (Math.random() * 100).toFixed(2) }}%</span
                >
                complete
              </div>
              <div class="col-span-12 flex gap-[2px] lg:mb-6">
                <div
                    v-for="item in Array(100)"
                    class="h-[22px] flex-1"
                    :class="`${
                    Math.random() > 0.5
                      ? 'bg-autonomi-red-300'
                      : Math.random() > 0.5
                      ? 'bg-autonomi-gray-500'
                      : 'bg-autonomi-blue-600'
                  }`"
                ></div>
              </div>
              <div
                  class="col-span-12 flex gap-10 text-sm text-autonomi-text-primary lg:hidden"
              >
                <div>{{ (Math.random() * 20).toFixed(2) }} GB</div>
                <div>{{ (Math.random() * 12).toFixed(0) }} of 265</div>
                <div>{{ (Math.random() * 100).toFixed(2) }}% complete</div>
              </div>
            </div>

            <!-- Row Menu -->
            <div class="hidden lg:block self-center">
              <i
                  class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                  @click="
                  $event => {
                    selectedDownloadItem = item;
                    handleToggleDownloadMenu($event);
                  }
                "
              />
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- MENUS -->
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
            {{ selectedFileItem.name }}
          </div>
        </div>

        <!--        <div class="py-3">-->
        <!--          <div>Type</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.type }}-->
        <!--          </div>-->
        <!--        </div>-->

        <!--        <div class="py-3">-->
        <!--          <div>Size</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.size }}-->
        <!--          </div>-->
        <!--        </div>-->

        <div class="py-3">
          <div>Size</div>
          <div class="text-autonomi-text-primary">
            {{ selectedFileItem?.metadata?.size }}
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

        <!--        <div class="py-3">-->
        <!--          <div>Opened</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.opened }}-->
        <!--          </div>-->
        <!--        </div>-->

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

    <!-- Upload Progress Modal -->
    <DialogInvoice
      :visible="showUploadModal"
      :current-step="currentUploadStep"
      :steps="uploadSteps"
      :quote-data="quoteData"
      :error="uploadError"
      @close-modal="handleCloseUploadModal"
      @cancel-upload="handleCancelUploadModal"
      @show-notify="emit('show-notify', $event)"
      @hide-notify="emit('hide-notify')"
    />
  </div>
</template>
