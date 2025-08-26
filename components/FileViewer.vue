<script lang="ts" setup>
import {useFileStore} from '~/stores/files';
import {useLocalFilesStore} from '~/stores/localFiles';
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
// Remove direct plugin import - we'll use the backend command instead

const toast = useToast();
const fileStore = useFileStore();
const localFilesStore = useLocalFilesStore();
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
  loadingArchives,
} = storeToRefs(fileStore);

const {
  pendingLocalStructure,
  currentDirectory: localCurrentDirectory,
  currentDirectoryFiles: localCurrentDirectoryFiles,
  rootDirectory: localRootDirectory,
  localFiles,
  failedArchives: localFailedArchives,
  loadingArchives: localLoadingArchives,
} = storeToRefs(localFilesStore);
const {uploadProgress} = storeToRefs(uploadStore);
const userStore = useUserStore();
const {query} = storeToRefs(userStore);

const activeTab = ref(0); // 0: Files, 1: Local Files, 2: Uploads, 3: Downloads
const viewTypeVault = ref<'grid' | 'list'>('list');
const breadcrumbs = ref<any[]>([]);
const isVisibleFileInfo = ref(false);
const refFilesMenu = ref();
const refFilesViewMenu = ref();
const refDownloadMenu = ref();
const refUploadMenu = ref();
const refUploadDropdown = ref();
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
const localBreadcrumbs = ref<any[]>([]);

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
      command: () => {
        handleDownloadFile(file);
        refFilesMenu.value.hide();
      },
    });
  } else if (file?.path) {
    // Only show download for files (not folders)
    items.push({
      label: 'Download',
      icon: 'pi pi-download',
      command: () => {
        handleDownloadFile(file);
        refFilesMenu.value.hide();
      },
    });
  }

  if (file?.path) {
    // Only show info for files (not folders)
    items.push({
      label: 'Info',
      icon: 'pi pi-info-circle',
      command: () => {
        isVisibleFileInfo.value = true;
        refFilesMenu.value.hide();
      },
    });
  }

  return items;
});

const menuUploads = computed(() => {
  const upload = selectedUploadItem.value;
  const items = [];

  if (upload?.status === 'failed') {
    items.push({
      label: 'Retry',
      icon: 'pi pi-refresh',
      command: () => {
        if (selectedUploadItem.value) {
          uploadsStore.retryUpload(selectedUploadItem.value.id);
        }
      },
    });
  }

  if (['pending', 'processing', 'encrypting', 'quoting', 'payment', 'uploading'].includes(upload?.status)) {
    items.push({
      label: 'Cancel',
      icon: 'pi pi-ban',
      command: () => {
        if (selectedUploadItem.value) {
          uploadsStore.cancelUpload(selectedUploadItem.value.id);
        }
      },
    });
  }

  items.push({
    label: 'Remove',
    icon: 'pi pi-times',
    command: () => {
      if (selectedUploadItem.value) {
        uploadsStore.removeUpload(selectedUploadItem.value.id);
      }
    },
  });

  return items;
});

const menuDownloads = computed(() => {
  const download = selectedDownloadItem.value;
  const items = [];

  if (download?.status === 'failed') {
    items.push({
      label: 'Retry',
      icon: 'pi pi-refresh',
      command: () => {
        if (selectedDownloadItem.value) {
          // Remove the failed download entry and start a new download
          const failedDownload = selectedDownloadItem.value;
          console.log('>>> Retry download - failedDownload:', failedDownload);

          downloadsStore.removeDownload(failedDownload.id);
          // Use the stored file object to retry download
          if (failedDownload.fileObject) {
            console.log('>>> Retry download - calling handleDownloadFile with stored fileObject');
            handleDownloadFile(failedDownload.fileObject);
          } else {
            console.log('>>> Retry download - no fileObject stored in failed download');
          }
        }
      },
    });
  }

  if (['pending', 'loading', 'downloading'].includes(download?.status)) {
    items.push({
      label: 'Cancel',
      icon: 'pi pi-ban',
      command: () => {
        if (selectedDownloadItem.value) {
          downloadsStore.cancelDownload(selectedDownloadItem.value.id);
        }
      },
    });
  }

  if (download?.status === 'completed' && download?.downloadPath) {
    items.push({
      label: 'Show in File Manager',
      icon: 'pi pi-folder-open',
      command: () => {
        if (selectedDownloadItem.value?.downloadPath) {
          showInFileManager(selectedDownloadItem.value.downloadPath);
          refDownloadMenu.value.hide();
        }
      },
    });
  }

  items.push({
    label: 'Remove',
    icon: 'pi pi-times',
    command: () => {
      if (selectedDownloadItem.value) {
        downloadsStore.removeDownload(selectedDownloadItem.value.id);
        refDownloadMenu.value.hide();
      }
    },
  });

  return items;
});

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

const menuUploadOptions = computed(() => [
  {
    label: 'Upload Files',
    icon: 'pi pi-file',
    command: openPickerAndUploadFiles,
    disabled: isUploading.value,
  },
  {
    label: 'Upload Folder',
    icon: 'pi pi-folder',
    command: openFolderPickerAndUploadFiles,
    disabled: isUploading.value,
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

// Combine regular files, failed archives, and loading archives (only in root directory)
const combinedFiles = computed(() => {
  const regularFiles = filteredFiles.value || [];

  // Only show failed and loading archives in the root directory
  const isRootDirectory = fileStore.currentDirectory?.name === 'Root';

  const failedArchiveFiles = isRootDirectory ? failedArchives.value.map(archive => ({
    name: archive.name,
    is_failed_archive: true,
    is_private: archive.is_private,
    is_loaded: false,
    is_loading: false,
    load_error: true,
    path: `failed-archive://${archive.name}`,
    metadata: {}
  })) : [];

  const loadingArchiveFiles = isRootDirectory ? loadingArchives.value.map(archive => ({
    name: archive.name,
    is_loading_archive: true,
    is_private: archive.is_private,
    is_loaded: false,
    is_loading: true,
    load_error: false,
    path: `loading-archive://${archive.name}`,
    metadata: {}
  })) : [];

  return [...regularFiles, ...failedArchiveFiles, ...loadingArchiveFiles];
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

    // Generate archive name
    const archiveName = files.length === 1
        ? files[0].name  // Single file: use filename
        : `${files.length}_files_${Date.now()}`; // Multiple files: use count and timestamp

    // Start the upload process
    await invoke("upload_files", {
      files,
      archiveName,
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

    // Check if we already have access_data from vault structure
    if (file.access_data && !file.file_access) {
      // access_data is already in the correct PublicOrPrivateFile format
      fileData = {
        ...file,
        file_access: file.access_data,
        is_loaded: true
      };
    } else if (!file.is_loaded && !file.is_loading && !file.file_access) {
      downloadsStore.updateDownload(downloadId, {status: 'loading'});

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

    downloadsStore.updateDownload(downloadId, {status: 'downloading'});

    try {
      // Get custom download path from settings, fallback to default
      const appData = await invoke('app_data') as any;
      const downloadsPath = appData.download_path || await downloadDir();
      console.log('Downloads path:', downloadsPath);
      const uniquePath = await invoke('get_unique_download_path', {
        downloadsPath,
        filename: fileName
      }) as string;
      console.log('Unique path:', uniquePath);

      console.log('Download fileData.file_access:', fileData.file_access);

      if (fileData.file_access.Private) {
        console.log('Downloading private file with dataMap:', fileData.file_access.Private);
        // Convert Vue Proxy to plain object
        const dataMap = JSON.parse(JSON.stringify(fileData.file_access.Private));
        await invoke('download_private_file', {
          dataMap: dataMap,
          toDest: uniquePath,
        });
      } else if (fileData.file_access.Public) {
        console.log('Downloading public file with addr:', fileData.file_access.Public);
        // Convert Vue Proxy to plain object  
        const addr = JSON.parse(JSON.stringify(fileData.file_access.Public));
        await invoke('download_public_file', {
          addr: addr,
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

      // Show a toast notification with action button
      console.log('>>> Adding download success toast for:', finalFileName, 'at path:', uniquePath);

      toast.add({
        severity: 'success',
        summary: 'Download Complete',
        detail: `File saved as: ${finalFileName}`,
        life: 8000,
        group: 'download-success',
        data: {
          filePath: uniquePath
        }
      });
    } catch (error: any) {
      console.error('Download error:', error);
      console.error('Error message:', error.message);
      console.error('Error details:', error);

      downloadsStore.updateDownload(downloadId, {
        status: 'failed',
        error: error.message || 'Download failed',
        completedAt: new Date()
      });
      toast.add({
        severity: 'error',
        summary: 'Download Failed',
        detail: `Failed to download the file: ${error.message || 'Unknown error'}`,
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

const formatUploadDuration = (startTime: Date, endTime?: Date): string => {
  const start = new Date(startTime);
  const end = endTime ? new Date(endTime) : new Date();
  const durationMs = end.getTime() - start.getTime();

  const seconds = Math.floor(durationMs / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
};

const secondsToDate = (seconds: number): Date => {
  return new Date(seconds * 1000);
};

const showInFileManager = async (filePath: string) => {
  try {
    console.log('>>> showInFileManager called with path:', filePath);

    // Call the Rust backend command to reveal the file in the file manager
    await invoke('show_item_in_file_manager', {path: filePath});

    // No success toast - the file manager opening is feedback enough
  } catch (error: any) {
    console.error('Failed to show file in file manager:', error);
    console.error('Error details:', error.message, error.stack);
    toast.add({
      severity: 'error',
      summary: 'Failed to Open',
      detail: `Could not open file location: ${error.message || error || 'Unknown error'}`,
      life: 3000,
    });
  }
};

// Set up event listeners early, before component mount
const setupEventListeners = async () => {
  const {listen} = await import("@tauri-apps/api/event");

  // Listen for vault updates from streaming
  await listen("vault-update", (event: any) => {
    console.log(">>> Received vault-update event:", event.payload);
    fileStore.handleVaultUpdate(event.payload);
  });

  // Listen for local file updates from streaming
  await listen("local-update", (event: any) => {
    console.log(">>> Received local-update event:", event.payload);
    localFilesStore.handleLocalUpdate(event.payload);
  });

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
    console.log(">>> Upload progress event:", payload.type, payload);

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
          console.log(`>>> Updating upload ${currentUploadId.value} with chunks: ${payload.chunks_uploaded}/${payload.total_chunks}, progress: ${progress}%`);
          // NOW the upload becomes active after payment completion
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'uploading',
            chunksUploaded: payload.chunks_uploaded || 0,
            totalChunks: payload.total_chunks || 0,
            progress
          });
        } else {
          console.log(">>> No currentUploadId when handling Uploading event");
        }
        if (showUploadModal.value) {
          // If we reach uploading, the payment was approved
          updateStepStatus('quoting', 'completed', 'Quote received');
          updateStepStatus('payment-request', 'completed', 'Payment authorized');

          // Close modal and show progress table (but keep upload ID active)
          showUploadModal.value = false;
          // Switch to uploads tab to show the active upload
          activeTab.value = 2;
          // Don't call handleCancelUploadModal() here - we need currentUploadId for subsequent events
          pendingUploadFiles.value = null;
          uploadSteps.value = [];
          currentUploadStep.value = '';
          uploadError.value = '';
          quoteData.value = null;
        }
        break;

      case "Completed":
        console.log(">>> Upload completed event received", payload);
        uploadStore.completeUpload();
        if (currentUploadId.value) {
          console.log(`>>> Marking upload ${currentUploadId.value} as completed`);
          uploadsStore.updateUpload(currentUploadId.value, {
            status: 'completed',
            progress: 100,
            completedAt: new Date()
          });
          // Clear the current upload ID now that upload is truly complete
          currentUploadId.value = null;
        } else {
          console.log(">>> No currentUploadId when handling Completed event - checking for active uploads");
          // If no currentUploadId but we have active uploads, mark the most recent one as completed
          const activeUploads = uploadsStore.activeUploads;
          if (activeUploads.length > 0) {
            const mostRecentUpload = activeUploads[0]; // They're sorted by creation time
            console.log(`>>> Marking most recent active upload ${mostRecentUpload.id} as completed`);
            uploadsStore.updateUpload(mostRecentUpload.id, {
              status: 'completed',
              progress: 100,
              completedAt: new Date()
            });
          }
        }
        // Auto-refresh files after upload completion
        setTimeout(() => {
          fileStore.getAllFiles();
          uploadStore.resetUpload();
          // Also refresh local files if on that tab
          if (activeTab.value === 1) {
            loadLocalFiles();
          }
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
          // Clear the current upload ID since upload failed
          currentUploadId.value = null;
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

// Function to load local files structure
const loadLocalFiles = async () => {
  try {
    await localFilesStore.getLocalStructure();
  } catch (error) {
    console.error('Failed to load local files:', error);
    toast.add({
      severity: 'error',
      summary: 'Failed to Load Local Files',
      detail: 'Could not retrieve local files',
      life: 3000,
    });
  }
};

// Auto-detect stuck uploads (runs every 30 seconds)
let stuckUploadCheckInterval: ReturnType<typeof setInterval> | null = null;

// Watch for tab changes to load local files when needed
watch(activeTab, (newTab) => {
  if (newTab === 1 && !localRootDirectory.value) {
    loadLocalFiles();
  }
});

// Get current local directory files for display (similar to vault files)
const localDirectoryFiles = computed(() => {
  if (!localCurrentDirectory.value) {
    return [];
  }
  return localCurrentDirectoryFiles.value || [];
});

// Combine local files with loading and failed archive states (similar to vault files)
const combinedLocalFiles = computed(() => {
  const regularFiles = localDirectoryFiles.value || [];

  // Only show failed and loading archives in the root directory
  const isRootDirectory = localCurrentDirectory.value === localRootDirectory.value;

  const failedArchiveFiles = isRootDirectory ? localFailedArchives.value.map(archive => ({
    name: archive.name,
    is_failed_archive: true,
    is_private: archive.is_private,
    is_loaded: false,
    is_loading: false,
    load_error: true,
    path: `failed-archive://${archive.address}`,
    address: archive.address,
    metadata: {}
  })) : [];

  const loadingArchiveFiles = isRootDirectory ? localLoadingArchives.value.map(archive => ({
    name: archive.name,
    is_loading_archive: true,
    is_private: archive.is_private,
    is_loaded: false,
    is_loading: true,
    load_error: false,
    path: `loading-archive://${archive.address}`,
    address: archive.address,
    metadata: {}
  })) : [];

  return [...regularFiles, ...failedArchiveFiles, ...loadingArchiveFiles];
});

// Handle local directory/file navigation (similar to vault files)
const handleLocalChangeDirectory = (target: any) => {
  if (!target?.children) {
    return;
  } else {
    localBreadcrumbs.value.push(target);
    localFilesStore.changeDirectory(target);
  }
};

// Handle local file name click
const handleLocalFileNameClick = (file: any) => {
  if (file.path) {
    selectedFileItem.value = file;
    isVisibleFileInfo.value = true;
  }
};

// Go back in local directory
const handleLocalGoBack = (target: any) => {
  localBreadcrumbs.value.pop();
  localFilesStore.changeDirectory(target);
};

// Handle local breadcrumb click
const handleLocalBreadcrumbClick = (crumb: any) => {
  const index = localBreadcrumbs.value.findIndex(breadcrumb => breadcrumb === crumb);
  localBreadcrumbs.value = localBreadcrumbs.value.slice(0, index + 1);
  localFilesStore.changeDirectory(crumb);
};

onMounted(async () => {
  try {
    fileStore.getAllFiles();
  } catch (err) {
    console.log('>>> Error getting files: ', err);
  }

  // Start checking for stuck uploads every 30 seconds
  stuckUploadCheckInterval = setInterval(() => {
    const now = new Date();
    const tenMinutesAgo = new Date(now.getTime() - 10 * 60 * 1000); // 10 minutes ago

    uploadsStore.activeUploads.forEach(upload => {
      const createdAt = new Date(upload.createdAt);
      // If an upload has been active for more than 10 minutes without completion, mark it as completed
      if (createdAt < tenMinutesAgo && upload.status === 'uploading' && upload.progress >= 90) {
        console.log(`>>> Auto-completing stuck upload ${upload.id} (created ${createdAt}, status: ${upload.status}, progress: ${upload.progress}%)`);
        uploadsStore.updateUpload(upload.id, {
          status: 'completed',
          progress: 100,
          completedAt: new Date()
        });
      }
    });
  }, 30000); // Check every 30 seconds
});

onUnmounted(() => {
  if (stuckUploadCheckInterval) {
    clearInterval(stuckUploadCheckInterval);
  }
});
</script>

<template>
  <div>
    <!-- Tab System -->
    <div>
      <!-- Upload Controls -->
      <div class="mx-[6rem] flex items-center justify-between mb-6">
        <div class="flex gap-3">
          <!-- Empty left side -->
        </div>

        <div class="flex items-center gap-3">
          <div
              v-if="(currentDirectory?.parent) || (activeTab === 1 && localCurrentDirectory?.parent)"
              class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
              @click="activeTab === 1 ? handleLocalGoBack(localCurrentDirectory.parent) : handleGoBack(currentDirectory.parent)"
          >
            <i class="pi pi-reply -scale-x-100 translate"/>
          </div>

          <div
              class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-blue-600 hover:bg-autonomi-blue-700 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
              @click="$event => { refUploadDropdown.toggle($event); }"
              :class="{ 'opacity-50 cursor-not-allowed': isUploading }"
              v-tooltip.bottom="'Upload'"
          >
            <i class="pi pi-plus"/>
          </div>

          <div
              v-if="activeTab === 0 || activeTab === 1"
              class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300 dark:bg-white dark:text-autonomi-blue-600 dark:hover:bg-white/70"
              v-tooltip.bottom="activeTab === 0 ? 'Refresh vault files' : 'Refresh local files'"
              @click="activeTab === 0 ? fileStore.getAllFiles() : loadLocalFiles()"
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

      <TabView v-model:activeIndex="activeTab">
        <!-- Vault Tab -->
        <TabPanel header="Vault" :value="0">
          <!-- Breadcrumbs -->
          <div
              v-if="breadcrumbs?.length > 0"
              class="mx-[6rem] flex gap-4 items-center text-sm font-semibold flex-wrap my-4"
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
              class="mt-6 overflow-y-auto overscroll-none"
              style="height: calc(100vh - 280px);"
          >
            <div class="grid grid-cols-12 font-semibold mb-10">
              <div
                  class="col-span-11 md:col-span-9 xl:col-span-8 pl-[6rem] text-autonomi-red-300"
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
                    @click="!file.is_loading_archive ? handleChangeDirectory(file) : null"
                    :class="{
                  'cursor-pointer': (!file.path || file.is_failed_archive) && !file.is_loading_archive,
                  'opacity-75': file.is_loading || file.is_loading_archive,
                  'opacity-75 bg-red-100 dark:bg-red-900/20 hover:bg-red-200': file.load_error || file.is_failed_archive,
                  'bg-blue-50 dark:bg-blue-900/20': file.is_loading_archive,
                  'hover:bg-white dark:hover:bg-[#8587c5]': !(file.load_error || file.is_failed_archive || file.is_loading_archive)
                }"
                >
                  <!-- Folder/File Name -->
                  <div
                      class="col-span-11 md:col-span-9 xl:col-span-8 pl-[6rem] flex items-center"
                  >
                    <template v-if="file.is_failed_archive">
                      <!-- This is a failed archive -->
                      <i class="pi pi-exclamation-triangle mr-4 text-red-500"/>
                      <i class="pi pi-box mr-2 text-red-500"/>
                      <span class="text-ellipsis overflow-hidden whitespace-nowrap text-red-600 dark:text-red-400">
                      {{ file.name }}
                    </span>
                    </template>
                    <template v-else-if="file.is_loading_archive">
                      <!-- This is a loading archive -->
                      <i class="pi pi-spinner pi-spin mr-4 text-blue-500"/>
                      <i class="pi pi-box mr-2 text-blue-500"/>
                      <span class="text-ellipsis overflow-hidden whitespace-nowrap text-blue-600 dark:text-blue-400">
                      {{ file.name }} (loading...)
                    </span>
                    </template>
                    <template v-else-if="file?.path">
                      <!-- This is a file -->
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
                          class="text-ellipsis overflow-hidden whitespace-nowrap cursor-pointer"
                          @click.stop="handleFileNameClick(file)">
                      {{ file.name }}
                    </span>
                      <!-- Loading indicators for files -->
                      <i v-if="file.is_loading" class="pi pi-spinner pi-spin ml-2 text-sm text-blue-500"/>
                      <i v-else-if="file.load_error" class="pi pi-exclamation-triangle ml-2 text-sm text-red-500"
                         v-tooltip.top="'Failed to load file data'"/>
                    </template>
                    <template v-else>
                      <!-- This is a folder or archive -->
                      <i :class="file.isArchive ? 'pi pi-box mr-4 text-amber-600 dark:text-amber-400' : 'pi pi-folder mr-4'"/>
                      <span class="text-ellipsis overflow-hidden whitespace-nowrap">{{ file.name }}</span>
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
                  <template v-if="file.path && !file.is_failed_archive && !file.is_loading_archive">
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
          </div>

          <!-- Grid View -->
          <div
              v-else-if="viewTypeVault === 'grid'"
              class="mt-6 overflow-y-auto overscroll-none"
              style="height: calc(100vh - 280px);"
          >
            <div class="px-3 grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
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
                    class="aspect-square w-full text-autonomi-text-primary hover:bg-white rounded-lg hover:text-autonomi-text-secondary dark:bg-[#444565] dark:hover:bg-black/40 dark:hover:text-autonomi-text-primary-dark transition-all duration-500 p-3 border flex flex-col"
                    :class="{
                      'cursor-pointer': !file.is_loading_archive,
                      'cursor-default opacity-75': file.is_loading_archive,
                      'bg-blue-50 dark:bg-blue-900/20': file.is_loading_archive
                    }"
                    @click="!file.is_loading_archive ? handleChangeDirectory(file) : null"
                >
                  <template v-if="file.path && !file.is_loading_archive">
                    <!-- Menu -->
                    <div class="self-end mb-2">
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
                  <template v-else>
                    <div class="self-end mb-2 h-4"></div>
                  </template>

                  <div class="flex flex-col items-center justify-center flex-1 min-h-0">
                    <div class="flex-shrink-0 mb-3">
                      <i v-if="file.is_failed_archive" class="pi pi-exclamation-triangle text-3xl text-red-500"/>
                      <i v-else-if="file.is_loading_archive" class="pi pi-spinner pi-spin text-3xl text-blue-500"/>
                      <i v-else-if="file.path" class="pi pi-file text-3xl"/>
                      <i v-else
                         :class="file.isArchive ? 'pi pi-box text-3xl text-amber-600 dark:text-amber-400' : 'pi pi-folder text-3xl'"/>
                    </div>

                    <div class="w-full px-1 min-h-0">
                      <span
                          class="text-center text-xs block w-full cursor-pointer overflow-hidden text-ellipsis"
                          style="display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; word-break: break-word;"
                          :title="file.is_loading_archive ? `${file.name} (loading...)` : file.name"
                          @click.stop="file.path ? handleFileNameClick(file) : null"
                      >
                        {{ file.is_loading_archive ? `${file.name} (loading...)` : file.name }}
                      </span>
                    </div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </TabPanel>

        <!-- Local Files Tab -->
        <TabPanel header="Local Files" :value="1">

          <!-- Local Files Breadcrumbs -->
          <div
              v-if="localBreadcrumbs?.length > 0"
              class="mx-[6rem] flex gap-4 items-center text-sm font-semibold flex-wrap my-4"
          >
            <div
                class="cursor-pointer transition-all duration-300 text-autonomi-text-secondary dark:text-autonomi-text-primary-dark"
                @click="handleLocalBreadcrumbClick(localRootDirectory)"
            >
              Local Files
            </div>
            <i class="text-xs pi pi-arrow-right text-autonomi-text-primary/70"/>

            <template v-for="(crumb, index) in localBreadcrumbs" :key="index">
              <div
                  :class="`cursor-pointer transition-all duration-300 ${
                  index === localBreadcrumbs.length - 1
                    ? 'text-autonomi-text-secondary'
                    : 'text-autonomi-text-primary/70'
                }`"
                  @click="handleLocalBreadcrumbClick(crumb)"
              >
                {{ crumb.name }}
              </div>
              <i
                  v-if="index !== localBreadcrumbs.length - 1"
                  class="text-xs pi pi-arrow-right text-autonomi-text-primary/70"
              />
            </template>
          </div>
          
          <!-- Files Table (List View) -->
          <div
              v-if="viewTypeVault === 'list'"
              class="mt-6 overflow-y-auto overscroll-none"
              style="height: calc(100vh - 280px);"
          >
            <div class="grid grid-cols-12 font-semibold mb-10">
              <div
                  class="col-span-11 md:col-span-9 xl:col-span-8 pl-[6rem] text-autonomi-red-300"
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
              <template v-if="combinedLocalFiles.length">
                <div
                    v-for="file in combinedLocalFiles"
                    :key="file.path || file.name"
                    class="grid grid-cols-subgrid col-span-12 h-11 items-center odd:bg-autonomi-gray-100 dark:odd:bg-[#5b5d87] dark:bg-[#444565] dark:text-autonomi-text-primary-dark"
                    @click="!file.is_loading_archive ? handleLocalChangeDirectory(file) : null"
                    :class="{
                      'cursor-pointer': (!file.path || file.is_failed_archive) && !file.is_loading_archive,
                      'opacity-75': file.is_loading || file.is_loading_archive,
                      'opacity-75 bg-red-100 dark:bg-red-900/20 hover:bg-red-200': file.load_error || file.is_failed_archive,
                      'bg-blue-50 dark:bg-blue-900/20': file.is_loading_archive,
                      'hover:bg-white dark:hover:bg-[#8587c5]': !(file.load_error || file.is_failed_archive || file.is_loading_archive)
                    }"
                >
                  <!-- Folder/File Name -->
                  <div
                      class="col-span-11 md:col-span-9 xl:col-span-8 pl-[6rem] flex items-center"
                  >
                    <template v-if="file.is_failed_archive">
                      <!-- This is a failed archive -->
                      <i class="pi pi-exclamation-triangle mr-4 text-red-500"/>
                      <i class="pi pi-box mr-2 text-red-500"/>
                      <span class="text-ellipsis overflow-hidden whitespace-nowrap text-red-600 dark:text-red-400">
                        {{ file.name }}
                      </span>
                    </template>
                    <template v-else-if="file.is_loading_archive">
                      <!-- This is a loading archive -->
                      <i class="pi pi-spinner pi-spin mr-4 text-blue-500"/>
                      <i class="pi pi-box mr-2 text-blue-500"/>
                      <span class="text-ellipsis overflow-hidden whitespace-nowrap text-blue-600 dark:text-blue-400">
                        {{ file.name }} (loading...)
                      </span>
                    </template>
                    <template v-else-if="file?.path">
                      <!-- This is a file -->
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
                          class="text-ellipsis overflow-hidden whitespace-nowrap cursor-pointer"
                          @click.stop="handleLocalFileNameClick(file)">
                        {{ file.name }}
                      </span>
                      <!-- Loading indicators for files -->
                      <i v-if="file.is_loading" class="pi pi-spinner pi-spin ml-2 text-sm text-blue-500"/>
                      <i v-else-if="file.load_error" class="pi pi-exclamation-triangle ml-2 text-sm text-red-500"
                         v-tooltip.top="'Failed to load file data'"/>
                    </template>
                    <template v-else>
                      <!-- This is a folder or archive -->
                      <i :class="file.isArchive ? 'pi pi-box mr-4 text-amber-600 dark:text-amber-400' : 'pi pi-folder mr-4'"/>
                      <span class="text-ellipsis overflow-hidden whitespace-nowrap">{{ file.name }}</span>
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
                  <template v-if="file.path && !file.is_failed_archive && !file.is_loading_archive">
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
                  <div v-if="pendingLocalStructure">
                    <i class="pi pi-spinner pi-spin mr-4"/>Loading local files...
                  </div>
                  <div v-else>No local files found. Files will appear here after you upload them.</div>
                </div>
              </template>
            </div>
          </div>

          <!-- Grid View -->
          <div
              v-else-if="viewTypeVault === 'grid'"
              class="mt-6 overflow-y-auto overscroll-none"
              style="height: calc(100vh - 280px);"
          >
            <div class="px-3 grid grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              <div v-if="!combinedLocalFiles.length" class="col-span-full p-8 text-center text-gray-500">
                <div v-if="pendingLocalStructure">
                  <i class="pi pi-spinner pi-spin mr-4"/>Loading local files...
                </div>
                <div v-else>No local files found. Files will appear here after you upload them.</div>
              </div>
              <template v-else>
                <div
                    v-for="file in combinedLocalFiles"
                    :key="file.path || file.name"
                    class="aspect-square w-full text-autonomi-text-primary hover:bg-white rounded-lg hover:text-autonomi-text-secondary dark:bg-[#444565] dark:hover:bg-black/40 dark:hover:text-autonomi-text-primary-dark transition-all duration-500 p-3 border flex flex-col"
                    :class="{
                      'cursor-pointer': !file.is_loading_archive,
                      'cursor-default opacity-75': file.is_loading_archive,
                      'bg-blue-50 dark:bg-blue-900/20': file.is_loading_archive
                    }"
                    @click="!file.is_loading_archive ? handleLocalChangeDirectory(file) : null"
                >
                  <template v-if="file.path && !file.is_loading_archive">
                    <!-- Menu -->
                    <div class="self-end mb-2">
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
                  <template v-else>
                    <div class="self-end mb-2 h-4"></div>
                  </template>

                  <div class="flex flex-col items-center justify-center flex-1 min-h-0">
                    <div class="flex-shrink-0 mb-3">
                      <i v-if="file.is_failed_archive" class="pi pi-exclamation-triangle text-3xl text-red-500"/>
                      <i v-else-if="file.is_loading_archive" class="pi pi-spinner pi-spin text-3xl text-blue-500"/>
                      <i v-else-if="file.path" class="pi pi-file text-3xl"/>
                      <i v-else
                         :class="file.isArchive ? 'pi pi-box text-3xl text-amber-600 dark:text-amber-400' : 'pi pi-folder text-3xl'"/>
                    </div>

                    <div class="w-full px-1 min-h-0">
                      <span
                          class="text-center text-xs block w-full cursor-pointer overflow-hidden text-ellipsis"
                          style="display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; word-break: break-word;"
                          :title="file.is_loading_archive ? `${file.name} (loading...)` : file.name"
                          @click.stop="file.path ? handleLocalFileNameClick(file) : null"
                      >
                        {{ file.is_loading_archive ? `${file.name} (loading...)` : file.name }}
                      </span>
                    </div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </TabPanel>

        <!-- Uploads Tab -->
        <TabPanel :header="`Uploads (${uploadsStore.activeUploads.length})`" :value="2">
          <div class="mx-[6rem] overflow-y-auto overscroll-none" style="height: calc(100vh - 280px);">
            <div class="space-y-4">
              <!-- Active Uploads -->
              <div v-if="uploadsStore.activeUploads.length > 0" class="space-y-2">
                <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
                  In Progress
                </h3>
                <div class="space-y-1">
                  <div
                      v-for="upload in uploadsStore.activeUploads"
                      :key="upload.id"
                      class="py-3"
                  >
                    <div class="flex items-center justify-between mb-2">
                      <div class="flex items-center gap-3 flex-1">
                        <!-- Icon -->
                        <i
                            class="pi text-autonomi-blue-600 dark:text-autonomi-blue-400"
                            :class="upload.totalFiles > 1 ? 'pi-folder' : 'pi-file'"
                        />

                        <!-- Name in blue -->
                        <span class="text-autonomi-blue-600 dark:text-autonomi-blue-400 font-medium">
                        {{ upload.name }}
                      </span>

                        <!-- Upload duration -->
                        <span class="text-sm text-gray-500 dark:text-gray-400 ml-auto mr-4">
                        {{ formatUploadDuration(upload.createdAt) }}
                      </span>
                      </div>

                      <!-- Menu icon -->
                      <i
                          class="pi pi-ellipsis-v cursor-pointer text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                          @click.stop="
                          selectedUploadItem = upload;
                          refUploadMenu.toggle($event);
                        "
                      />
                    </div>

                    <!-- Progress bar -->
                    <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                      <div
                          class="h-1.5 rounded-full transition-all duration-300"
                          :class="{
                          'bg-blue-500': upload.status === 'pending' || upload.status === 'processing',
                          'bg-purple-500': upload.status === 'encrypting',
                          'bg-orange-500': upload.status === 'quoting',
                          'bg-yellow-500': upload.status === 'payment',
                          'bg-green-500': upload.status === 'uploading'
                        }"
                          :style="`width: ${upload.progress}%`"
                      />
                    </div>
                  </div>
                </div>
              </div>

              <!-- Completed uploads -->
              <div v-if="uploadsStore.completedUploads.length > 0" class="space-y-2 mt-8">
                <div class="flex justify-between items-center mb-4">
                  <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark">
                    Completed
                  </h3>
                  <button
                      class="text-xs px-2 py-1 bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded transition-colors"
                      @click="uploadsStore.clearCompleted()"
                  >
                    Clear All
                  </button>
                </div>
                <div class="space-y-1">
                  <div
                      v-for="upload in uploadsStore.completedUploads.slice(0, 10)"
                      :key="upload.id"
                      class="py-2 flex items-center justify-between"
                  >
                    <div class="flex items-center gap-3">
                      <!-- Icon -->
                      <i
                          class="pi text-green-600 dark:text-green-400"
                          :class="upload.totalFiles > 1 ? 'pi-folder' : 'pi-file'"
                      />

                      <!-- Name -->
                      <span class="text-autonomi-text-primary dark:text-autonomi-text-primary-dark">
                      {{ upload.name }}
                    </span>

                      <!-- Duration -->
                      <span class="text-sm text-gray-500 dark:text-gray-400">
                      took {{ formatUploadDuration(upload.createdAt, upload.completedAt) }}
                    </span>
                    </div>

                    <!-- Menu icon -->
                    <i
                        class="pi pi-ellipsis-v cursor-pointer text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                        @click.stop="
                        selectedUploadItem = upload;
                        refUploadMenu.toggle($event);
                      "
                    />
                  </div>
                </div>
              </div>

              <!-- Failed uploads -->
              <div v-if="uploadsStore.failedUploads.length > 0" class="space-y-2 mt-8">
                <div class="flex justify-between items-center mb-4">
                  <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark">
                    Failed
                  </h3>
                  <button
                      class="text-xs px-2 py-1 bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded transition-colors"
                      @click="uploadsStore.clearFailed()"
                  >
                    Clear All
                  </button>
                </div>
                <div class="space-y-1">
                  <div
                      v-for="upload in uploadsStore.failedUploads.slice(0, 10)"
                      :key="upload.id"
                      class="py-2 flex items-center justify-between"
                  >
                    <div class="flex items-center gap-3 flex-1">
                      <!-- Icon -->
                      <i
                          class="pi text-red-600 dark:text-red-400"
                          :class="upload.totalFiles > 1 ? 'pi-folder' : 'pi-file'"
                      />

                      <!-- Name -->
                      <span class="text-autonomi-text-primary dark:text-autonomi-text-primary-dark">
                      {{ upload.name }}
                    </span>

                      <!-- Error message -->
                      <span class="text-sm text-red-600 dark:text-red-400" v-if="upload.error">
                      - {{ upload.error }}
                    </span>
                    </div>

                    <div class="flex items-center gap-2">
                      <i
                          class="pi pi-refresh cursor-pointer text-gray-400 hover:text-blue-500 transition-colors"
                          @click.stop="uploadsStore.retryUpload(upload.id)"
                          v-tooltip.top="'Retry upload'"
                      />
                      <i
                          class="pi pi-ellipsis-v cursor-pointer text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                          @click.stop="
                          selectedUploadItem = upload;
                          refUploadMenu.toggle($event);
                        "
                      />
                    </div>
                  </div>
                </div>
              </div>

              <div v-if="uploadsStore.sortedUploads.length === 0" class="text-center py-8 text-gray-500">
                No uploads yet
              </div>
            </div>
          </div>
        </TabPanel>

        <!-- Downloads Tab -->
        <TabPanel :header="`Downloads (${downloadsStore.activeDownloads.length})`" :value="3">
          <div class="mx-[6rem] overflow-y-auto overscroll-none" style="height: calc(100vh - 280px);">
            <div class="space-y-4">
              <!-- Active Downloads -->
              <div v-if="downloadsStore.activeDownloads.length > 0" class="space-y-2">
                <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark mb-4">
                  In Progress
                </h3>
                <div class="space-y-1">
                  <div
                      v-for="download in downloadsStore.activeDownloads"
                      :key="download.id"
                      class="py-3"
                  >
                    <div class="flex items-center justify-between mb-2">
                      <div class="flex items-center gap-3 flex-1">
                        <!-- Icon -->
                        <i class="pi pi-file text-autonomi-blue-600 dark:text-autonomi-blue-400"/>

                        <!-- Name in blue -->
                        <span class="text-autonomi-blue-600 dark:text-autonomi-blue-400 font-medium">
                        {{ download.fileName }}
                      </span>

                        <!-- Download duration -->
                        <span class="text-sm text-gray-500 dark:text-gray-400 ml-auto mr-4">
                        {{ formatUploadDuration(download.createdAt) }}
                      </span>
                      </div>

                      <!-- Menu icon -->
                      <i
                          class="pi pi-ellipsis-v cursor-pointer text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                          @click.stop="
                          selectedDownloadItem = download;
                          refDownloadMenu.toggle($event);
                        "
                      />
                    </div>

                    <!-- Progress bar -->
                    <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                      <div
                          class="h-1.5 rounded-full transition-all duration-300"
                          :class="{
                          'bg-blue-500': download.status === 'loading',
                          'bg-green-500': download.status === 'downloading'
                        }"
                          :style="`width: ${download.progress}%`"
                      />
                    </div>
                  </div>
                </div>
              </div>

              <!-- Completed Downloads -->
              <div v-if="downloadsStore.completedDownloads.length > 0" class="space-y-2 mt-8">
                <div class="flex justify-between items-center mb-4">
                  <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark">
                    Completed
                  </h3>
                  <button
                      class="text-xs px-2 py-1 bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded transition-colors"
                      @click="downloadsStore.clearCompleted()"
                  >
                    Clear All
                  </button>
                </div>
                <div class="space-y-1">
                  <div
                      v-for="download in downloadsStore.completedDownloads.slice(0, 10)"
                      :key="download.id"
                      class="py-2 flex items-center justify-between"
                  >
                    <div class="flex items-center gap-3">
                      <!-- Icon -->
                      <i class="pi pi-file text-green-600 dark:text-green-400"/>

                      <!-- Name -->
                      <span class="text-autonomi-text-primary dark:text-autonomi-text-primary-dark">
                      {{ download.fileName }}
                    </span>

                      <!-- Duration -->
                      <span class="text-sm text-gray-500 dark:text-gray-400">
                      took {{ formatUploadDuration(download.createdAt, download.completedAt) }}
                    </span>

                      <!-- Download location -->
                      <span class="text-sm text-gray-500 dark:text-gray-400" v-if="download.downloadPath">
                      - {{ download.downloadPath.split('/').pop() }}
                    </span>
                    </div>

                    <!-- Menu icon -->
                    <i
                        class="pi pi-ellipsis-v cursor-pointer text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                        @click.stop="
                        selectedDownloadItem = download;
                        refDownloadMenu.toggle($event);
                      "
                    />
                  </div>
                </div>
              </div>

              <!-- Failed Downloads -->
              <div v-if="downloadsStore.failedDownloads.length > 0" class="space-y-2 mt-8">
                <div class="flex justify-between items-center mb-4">
                  <h3 class="text-lg font-semibold text-autonomi-header-text dark:text-autonomi-text-primary-dark">
                    Failed
                  </h3>
                  <button
                      class="text-xs px-2 py-1 bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded transition-colors"
                      @click="downloadsStore.clearFailed()"
                  >
                    Clear All
                  </button>
                </div>
                <div class="space-y-1">
                  <div
                      v-for="download in downloadsStore.failedDownloads.slice(0, 10)"
                      :key="download.id"
                      class="py-2 flex items-center justify-between"
                  >
                    <div class="flex items-center gap-3 flex-1">
                      <!-- Icon -->
                      <i class="pi pi-file text-red-600 dark:text-red-400"/>

                      <!-- Name -->
                      <span class="text-autonomi-text-primary dark:text-autonomi-text-primary-dark">
                      {{ download.fileName }}
                    </span>

                      <!-- Error message -->
                      <span class="text-sm text-red-600 dark:text-red-400" v-if="download.error">
                      - {{ download.error }}
                    </span>
                    </div>

                    <div class="flex items-center gap-2">
                      <i
                          class="pi pi-refresh cursor-pointer text-gray-400 hover:text-blue-500 transition-colors"
                          @click.stop="() => {
                            // Remove the failed download entry and start a new download
                            console.log('>>> Inline retry - failedDownload:', download);
                            
                            downloadsStore.removeDownload(download.id);
                            // Use the stored file object to retry download
                            if (download.fileObject) {
                              console.log('>>> Inline retry - calling handleDownloadFile with stored fileObject');
                              handleDownloadFile(download.fileObject);
                            } else {
                              console.log('>>> Inline retry - no fileObject stored in failed download');
                            }
                          }"
                          v-tooltip.top="'Retry download'"
                      />
                      <i
                          class="pi pi-ellipsis-v cursor-pointer text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                          @click.stop="
                          selectedDownloadItem = download;
                          refDownloadMenu.toggle($event);
                        "
                      />
                    </div>
                  </div>
                </div>
              </div>

              <div v-if="downloadsStore.sortedDownloads.length === 0" class="text-center py-8 text-gray-500">
                No downloads yet
              </div>
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

    <!-- UPLOAD DROPDOWN POPOVER -->
    <Popover ref="refUploadDropdown" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
                v-for="item in menuUploadOptions"
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
            {{ selectedFileItem?.name }}
          </div>
        </div>

        <!-- Show type for local files -->
        <div v-if="selectedFileItem?.type" class="py-3">
          <div>Type</div>
          <div class="text-autonomi-text-primary">
            <template v-if="selectedFileItem.type === 'public_archive'">
              Public Archive
            </template>
            <template v-else-if="selectedFileItem.type === 'private_archive'">
              Private Archive
            </template>
            <template v-else-if="selectedFileItem.type === 'public_file'">
              Public File
            </template>
            <template v-else-if="selectedFileItem.type === 'private_file'">
              Private File
            </template>
          </div>
        </div>

        <!-- Show address for local files -->
        <div v-if="selectedFileItem?.address" class="py-3">
          <div>Address</div>
          <div class="text-autonomi-text-primary font-mono text-xs break-all">
            {{ selectedFileItem.address }}
          </div>
        </div>

        <div class="py-3">
          <div>Size</div>
          <div class="text-autonomi-text-primary">
            {{ selectedFileItem?.metadata?.size ? formatBytes(selectedFileItem.metadata.size) : 'Unknown' }}
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
                  : 'Unknown'
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
                  : 'Unknown'
            }}
          </div>
        </div>
      </div>
    </Drawer>
  </div>
</template>