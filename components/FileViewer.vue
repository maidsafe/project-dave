<script lang="ts" setup>
import {useFileStore} from '~/stores/files';
import {useLocalFilesStore} from '~/stores/localFiles';
import {useToast} from 'primevue/usetoast';
import {useConfirm} from 'primevue/useconfirm';
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
const confirm = useConfirm();
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
// Only tracks the upload ID for the modal dialog - not for all uploads
const modalUploadId = ref<string | null>(null);
// Store quote data per upload ID for concurrent uploads
const uploadQuotes = ref<Map<string, any>>(new Map());
const localBreadcrumbs = ref<any[]>([]);

// Vault removal loading state
const pendingVaultRemoval = ref(false);
const vaultRemovalItem = ref<{ name: string, isArchive: boolean } | null>(null);

// Upload modal functionality
const initializeUploadSteps = () => {
  uploadSteps.value = [
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

const handleCancelUploadModal = async () => {
  // Since uploads can only be cancelled before payment, just update UI state
  if (modalUploadId.value) {
    uploadsStore.updateUpload(modalUploadId.value, {
      status: 'failed',
      error: 'Upload cancelled',
      completedAt: new Date()
    });
  }

  // Reset modal state
  showUploadModal.value = false;
  pendingUploadFiles.value = null;
  uploadSteps.value = [];
  currentUploadStep.value = '';
  uploadError.value = '';
  quoteData.value = null;
  modalUploadId.value = null;
};

const initiatePaymentForUpload = (uploadId: string) => {
  console.log(">>> Initiating payment for upload:", uploadId);

  // Check if there's already a modal open
  if (showUploadModal.value) {
    toast.add({
      severity: "warn",
      summary: "Payment in Progress",
      detail: "Please complete the current payment first, then try again.",
      life: 5000,
    });
    return;
  }

  // Get stored quote data for this upload
  const storedQuote = uploadQuotes.value.get(uploadId);
  if (!storedQuote) {
    toast.add({
      severity: "error",
      summary: "Quote Not Found",
      detail: "Quote data not available for this upload. Please try uploading again.",
      life: 5000,
    });
    return;
  }

  // Check if payment is actually required
  if (!storedQuote.paymentRequired || !storedQuote.payments || storedQuote.payments.length === 0) {
    toast.add({
      severity: "info",
      summary: "No Payment Required",
      detail: "This upload doesn't require payment.",
      life: 3000,
    });
    return;
  }

  // Set this upload as the modal upload and prepare payment modal
  modalUploadId.value = uploadId;

  // Set quote data for the modal
  quoteData.value = storedQuote;

  // Initialize payment modal
  initializeUploadSteps();
  updateStepStatus('quoting', 'completed', 'Quote retrieved');
  updateStepStatus('payment-request', 'pending', 'Ready for payment...');

  // Show the payment modal
  showUploadModal.value = true;

  console.log(">>> Payment modal opened for upload:", uploadId);
};

const handleCloseUploadModal = () => {
  // Check if there are any active processing steps in the modal
  const hasActiveProcessing = uploadSteps.value.some(step => step.status === 'processing');

  // Also check if the modal upload is already in progress (beyond initial 'quoting' status)
  const modalUpload = modalUploadId.value ? uploadsStore.uploads.find(u => u.id === modalUploadId.value) : null;
  const uploadInProgress = modalUpload && ['uploading'].includes(modalUpload.status);

  console.log(">>> handleCloseUploadModal - hasActiveProcessing:", hasActiveProcessing, "uploadInProgress:", uploadInProgress, "modalUpload status:", modalUpload?.status);

  // Always allow modal to close, but only cancel upload if it's not in progress
  if (!hasActiveProcessing && !uploadInProgress) {
    console.log(">>> Modal closing - cancelling upload since it's not in progress");
    handleCancelUploadModal();
  } else {
    console.log(">>> Modal closing - keeping upload alive since it's in progress");
    // Just close the modal without cancelling the upload
    showUploadModal.value = false;
    // Clean up modal state but keep upload running
    pendingUploadFiles.value = null;
    uploadSteps.value = [];
    currentUploadStep.value = '';
    uploadError.value = '';
    quoteData.value = null;
    modalUploadId.value = null;
  }
};

const handlePayUpload = async () => {
  console.log(">>> handlePayUpload called with quoteData:", quoteData.value);
  console.log(">>> payments array:", quoteData.value?.payments);
  console.log(">>> payment_required:", quoteData.value?.payment_required);

  if (!quoteData.value?.payments || quoteData.value.payments.length === 0) {
    console.error("No payments data available - might be free upload");

    // If it's a free upload (no payments required), proceed directly
    if (quoteData.value?.paymentRequired === false) {
      console.log(">>> Free upload detected, proceeding without payment");

      // Update UI to show no payment needed
      updateStepStatus('payment-request', 'completed', 'No payment required');

      // Free upload - complete instantly
      if (modalUploadId.value) {
        console.log(">>> Free upload in payment modal - marking as completed instantly");

        // Determine completion message based on cost
        let completionMessage = 'Upload completed';
        if (quoteData.value?.totalCostNano === '0' || quoteData.value?.totalCostNano === 0) {
          completionMessage = 'Already uploaded';
          console.log(">>> Duplicate upload detected in payment modal (cost = 0) - using 'already uploaded' message");
        }

        // Mark upload as completed immediately
        uploadsStore.updateUpload(modalUploadId.value, {
          status: 'completed',
          progress: 100,
          completionMessage,
          completedAt: new Date()
        });

        // Clean up stored quote data
        uploadQuotes.value.delete(modalUploadId.value);

        // Trigger file refresh
        setTimeout(() => {
          fileStore.getAllFiles();
        }, 500);
      }

      showUploadModal.value = false;
      return;
    }

    // If payments are required but not available, it's an error
    console.error("Payments required but no payment data available");
    return;
  }

  try {
    console.log(">>> Processing payment for quote:", quoteData.value);
    console.log(">>> Wallet store state - checking wallet connection...");
    console.log(">>> Payments to process:", quoteData.value.payments);

    // The wallet connection check will happen inside payForQuotes method
    // If wallet is not connected, it should throw an error there

    // Update UI to show payment in progress
    updateStepStatus('payment-request', 'processing', 'Requesting wallet authorization...');

    // Show wallet payment notification
    emit("show-notify", {
      notifyType: "info",
      title: "Payment required",
      details: "Please approve the payment in your mobile wallet.",
    });

    // Process payment through wallet
    console.log(">>> Calling walletStore.payForQuotes...");

    // Use rawPayments if available, otherwise fall back to payments
    const quotes = quoteData.value.rawPayments;
    const txHashes = await walletStore.payForQuotes(quotes);
    console.log(">>> walletStore.payForQuotes completed successfully:", txHashes);
    console.log(">>> Payment successful, transaction hashes:", txHashes);

    // Hide wallet payment notification
    emit("hide-notify");

    // Update UI to show wallet payment successful
    updateStepStatus('payment-request', 'completed', 'Payment confirmed');

    console.log(">>> FILEVIEWER PAYMENT COMPLETED - notifying backend to proceed");

    // Confirm payment with backend to trigger upload execution
    if (modalUploadId.value) {
      try {
        await invoke("confirm_upload_payment", {
          uploadId: modalUploadId.value // Use the same ID throughout!
        });
        console.log(">>> Backend notified of payment confirmation");
      } catch (error) {
        console.error("Failed to confirm payment with backend:", error);
        updateStepStatus('payment-request', 'error', 'Failed to start upload');
        return;
      }
    }

    // Don't manually update upload status - wait for backend Started/Uploading events
    if (modalUploadId.value) {
      console.log(">>> PAYMENT COMPLETE - Waiting for backend to start upload:", modalUploadId.value);
    }

    // The upload will proceed automatically since payment is confirmed
    // Close modal after successful payment
    console.log(">>> PAYMENT COMPLETE - Closing modal, switching to uploads tab");
    console.log(">>> showUploadModal.value before:", showUploadModal.value);
    showUploadModal.value = false;
    console.log(">>> showUploadModal.value after:", showUploadModal.value);
    activeTab.value = 2; // Switch to uploads tab
    console.log(">>> Current active tab:", activeTab.value);

    // Clean up modal state
    pendingUploadFiles.value = null;
    uploadSteps.value = [];
    currentUploadStep.value = '';
    uploadError.value = '';
    quoteData.value = null;
    modalUploadId.value = null;

  } catch (error) {
    console.error("Payment failed:", error);
    console.error("Error details:", {
      message: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
      name: error instanceof Error ? error.name : undefined,
      cause: error instanceof Error ? error.cause : undefined
    });
    updateStepStatus('payment-request', 'error', 'Payment failed');

    // Hide wallet payment notification on error
    emit("hide-notify");

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
  } else if (file?.load_error && !file?.is_failed_archive) {
    items.push({
      label: 'Retry Download',
      icon: 'pi pi-refresh',
      command: () => {
        handleDownloadFile(file);
        refFilesMenu.value.hide();
      },
    });
  } else if (file?.path && !file?.is_failed_archive) {
    // Only show download for files (not folders or failed archives)
    items.push({
      label: 'Download',
      icon: 'pi pi-download',
      command: () => {
        handleDownloadFile(file);
        refFilesMenu.value.hide();
      },
    });
  }

  if (file?.path && !file?.is_failed_archive) {
    // Only show info for files (not folders or failed archives)
    items.push({
      label: 'Info',
      icon: 'pi pi-info-circle',
      command: () => {
        isVisibleFileInfo.value = true;
        refFilesMenu.value.hide();
      },
    });

    // Show data address for public files
    if (file?.file_access?.Public || file?.access_data?.Public || file?.type === 'public_file' || file?.type === 'public_archive') {
      items.push({
        label: 'Data Address',
        icon: 'pi pi-clipboard',
        command: () => {
          handleCopyDataAddress(file);
          refFilesMenu.value.hide();
        },
      });
    }
  }

  // Add remove option for vault files only (not local files, loading archives)

  if (activeTab.value === 0 && !file?.is_loading_archive) {
    // Determine if this is an archive file (file is inside an archive)
    const isArchiveFile = file?.archive_name && file?.archive_name !== '';

    if (file?.is_failed_archive) {
      // Failed archive - remove from vault
      items.push({
        label: 'Remove Archive from Vault',
        icon: 'pi pi-trash',
        class: 'text-red-600',
        command: () => {
          handleRemoveFromVault(file, false);
          refFilesMenu.value.hide();
        },
      });
    } else if (isArchiveFile) {
      // File within an archive - remove the whole archive
      items.push({
        label: 'Remove Archive from Vault',
        icon: 'pi pi-trash',
        class: 'text-red-600',
        command: () => {
          handleRemoveFromVault(file, true);
          refFilesMenu.value.hide();
        },
      });
    } else if (file?.isArchive) {
      // Archive folder itself - remove the archive
      items.push({
        label: 'Remove Archive from Vault',
        icon: 'pi pi-trash',
        class: 'text-red-600',
        command: () => {
          handleRemoveFromVault(file, false);
          refFilesMenu.value.hide();
        },
      });
    } else if (file?.path) {
      // Individual file - remove just the file
      items.push({
        label: 'Remove from Vault',
        icon: 'pi pi-trash',
        class: 'text-red-600',
        command: () => {
          handleRemoveFromVault(file, false);
          refFilesMenu.value.hide();
        },
      });
    }
  }

  return items;
});

const menuUploads = computed(() => {
  const upload = selectedUploadItem.value;
  const items = [];


  // Add "Pay Now" option for uploads in quoting status that need payment
  if (upload?.status === 'quoting') {
    // Check if this upload has quote data that requires payment
    const storedQuote = uploadQuotes.value.get(upload.id);
    const needsPayment = storedQuote?.paymentRequired && storedQuote?.payments && storedQuote.payments.length > 0;

    if (needsPayment) {
      items.push({
        label: 'Pay Now',
        icon: 'pi pi-credit-card',
        command: () => {
          if (selectedUploadItem.value) {
            // Trigger payment modal for this upload
            initiatePaymentForUpload(selectedUploadItem.value.id);
          }
        },
      });
    }
  }

  if (['quoting', 'uploading'].includes(upload?.status)) {
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
  },
  {
    label: 'Upload Folder',
    icon: 'pi pi-folder',
    command: openFolderPickerAndUploadFiles,
  },
]);

// Upload functions
const emit = defineEmits(["show-notify", "hide-notify"]);

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
  const isRootDirectory = currentDirectory.value === rootDirectory.value;

  const failedArchiveFiles = isRootDirectory ? failedArchives.value
      .filter(archive => !query.value || archive.name.toLowerCase().includes(query.value.toLowerCase()))
      .map(archive => ({
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

  const loadingArchiveFiles = isRootDirectory ? loadingArchives.value
      .filter(archive => !query.value || archive.name.toLowerCase().includes(query.value.toLowerCase()))
      .map(archive => ({
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

  return [...regularFiles, ...failedArchiveFiles, ...loadingArchiveFiles].sort((a, b) => {
    // Sort by name, putting directories first, then files
    if (a.is_directory !== b.is_directory) {
      return a.is_directory ? -1 : 1;
    }
    return a.name.localeCompare(b.name, undefined, {numeric: true, sensitivity: 'base'});
  });
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
  await uploadFiles(files, true);
};

const uploadFiles = async (files: Array<{ path: string, name: string }>, isFolder: boolean = false) => {
  try {
    let vaultKeySignature = await walletStore.getVaultKeySignature();

    // Create upload entry in the store (but keep it pending until payment)
    const frontendUploadId = uploadsStore.createUpload(files);
    console.log(">>> Created upload with ID:", frontendUploadId);
    console.log(">>> Upload in store:", uploadsStore.uploads.find(u => u.id === frontendUploadId));
    console.log(">>> Active uploads after creation:", uploadsStore.activeUploads.length);

    // Check if another upload modal is already open
    if (showUploadModal.value && modalUploadId.value) {
      console.log(">>> Another upload modal is already open. Upload created but will proceed without modal UI.");
      // The upload will proceed in the background and get processed when it receives the quote event
      // This allows multiple uploads to be started even if one modal is open
    } else {
      // Set this upload as the modal upload and show the modal
      modalUploadId.value = frontendUploadId;

      // Initialize and show modal, then start the quoting process
      initializeUploadSteps();
      pendingUploadFiles.value = {files, vaultKeySignature, isFolder};
      showUploadModal.value = true;
    }

    // Start by getting the quote only - no actual upload yet
    console.log(">>> FILEVIEWER STARTING UPLOAD WITH NEW SYSTEM");

    // Update step to show quoting in progress
    updateStepStatus('quoting', 'processing', 'Getting storage cost estimate...');

    // Generate archive name
    let archiveName: string;
    if (isFolder && files.length === 1) {
      // Folder upload: use folder name
      archiveName = files[0].name;
    } else {
      // Single file or multiple files: no archive name
      archiveName = "";
    }

    // Start upload with frontend-generated ID - much simpler!
    await invoke("start_upload", {
      files,
      archiveName,
      vaultKeySignature,
      uploadId: frontendUploadId, // Pass our ID to backend
    });

    console.log(">>> Upload started with ID:", frontendUploadId);

    // The upload-quote event will be emitted by the backend and handled by the event listener


  } catch (error: any) {
    emit("hide-notify");
    console.error("Error in uploadFiles:", error);

    // Update modal if showing
    if (showUploadModal.value) {
      uploadError.value = error.message || "Unknown error occurred";
      updateStepStatus(currentUploadStep.value || 'processing', 'error', error.message);
    }

    // Update upload status to failed
    if (modalUploadId.value) {
      uploadsStore.updateUpload(modalUploadId.value, {
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

const cancelUpload = async (uploadId: string) => {
  try {
    await invoke("cancel_upload", {uploadId});
    toast.add({
      severity: "info",
      summary: "Upload cancelled",
      detail: "The upload has been cancelled",
      life: 3000,
    });
  } catch (error: any) {
    console.error("Error cancelling upload:", error);
    toast.add({
      severity: "error",
      summary: "Error cancelling upload",
      detail: error.message,
      life: 3000,
    });
  }
};

const handleCopyDataAddress = async (file: any) => {
  try {
    let dataAddress = '';

    // Extract data address based on file structure
    if (file?.file_access?.Public) {
      // For vault files, Public contains the data address
      dataAddress = file.file_access.Public;
    } else if (file?.access_data?.Public) {
      // Alternative structure
      dataAddress = file.access_data.Public;
    } else if (file?.address) {
      // For local files, use the address directly
      dataAddress = file.address;
    }

    if (dataAddress) {
      await navigator.clipboard.writeText(dataAddress);
      toast.add({
        severity: 'success',
        summary: 'Copied',
        detail: 'Data address copied to clipboard',
        life: 2000,
      });
    } else {
      toast.add({
        severity: 'error',
        summary: 'Error',
        detail: 'Could not find data address',
        life: 3000,
      });
    }
  } catch (error) {
    console.error('Failed to copy data address:', error);
    toast.add({
      severity: 'error',
      summary: 'Error',
      detail: 'Failed to copy to clipboard',
      life: 3000,
    });
  }
};

const handleCopySecretKey = async (file: any) => {
  try {
    let secretKey = '';

    // Extract secret key based on file structure
    if (file?.file_access?.Private) {
      // For vault files, Private contains the secret key/datamap
      // Convert to hex string if it's an array
      if (Array.isArray(file.file_access.Private)) {
        secretKey = '0x' + file.file_access.Private.map((byte: number) =>
            byte.toString(16).padStart(2, '0')
        ).join('');
      } else {
        secretKey = file.file_access.Private;
      }
    } else if (file?.access_data?.Private) {
      // Alternative structure
      if (Array.isArray(file.access_data.Private)) {
        secretKey = '0x' + file.access_data.Private.map((byte: number) =>
            byte.toString(16).padStart(2, '0')
        ).join('');
      } else {
        secretKey = file.access_data.Private;
      }
    } else if (file?.address && (file?.type === 'private_file' || file?.type === 'private_archive')) {
      // For local private files, use the address
      secretKey = file.address;
    }

    if (secretKey) {
      await navigator.clipboard.writeText(secretKey);
      toast.add({
        severity: 'success',
        summary: 'Copied',
        detail: 'Data access copied to clipboard',
        life: 2000,
      });
    } else {
      toast.add({
        severity: 'error',
        summary: 'Error',
        detail: 'Could not find data access',
        life: 3000,
      });
    }
  } catch (error) {
    console.error('Failed to copy secret key:', error);
    toast.add({
      severity: 'error',
      summary: 'Error',
      detail: 'Failed to copy to clipboard',
      life: 3000,
    });
  }
};

const handleRemoveFromVault = async (file: any, isArchiveFile: boolean) => {
  // Determine what we're removing
  const fileName = file.name;
  const isArchive = file.isArchive || isArchiveFile || file.is_failed_archive;

  // Get archive address if removing an archive or a file within an archive
  let archiveAddress = null;
  if (file.is_failed_archive && file.address) {
    // This is a failed archive - use its address directly
    archiveAddress = file.address;
  } else if (file.isArchive && file.archive?.address) {
    // This is an archive folder itself - get address from the archive object
    archiveAddress = file.archive.address;
  } else if (isArchiveFile && file.archive_address) {
    // This is a file within an archive
    archiveAddress = file.archive_address;
  }

  // Show confirmation dialog using PrimeVue's confirm service
  const message = isArchive
      ? `Are you sure you want to remove the archive "${fileName}" from your vault? This will remove ALL files within this archive. This action cannot be undone.`
      : `Are you sure you want to remove "${fileName}" from your vault? This action cannot be undone.`;

  // Use PrimeVue's confirm service for better reliability
  confirm.require({
    message: message,
    header: 'Confirm Removal',
    icon: 'pi pi-exclamation-triangle',
    rejectProps: {
      label: 'Cancel',
      severity: 'secondary',
      outlined: true
    },
    acceptProps: {
      label: 'Remove',
      severity: 'danger'
    },
    accept: async () => {
      try {
        // Set loading state with item information
        vaultRemovalItem.value = {name: fileName, isArchive};
        pendingVaultRemoval.value = true;

        // Get vault key signature
        const walletStore = useWalletStore();
        const vaultKeySignature = await walletStore.getVaultKeySignature();


        // Call the remove function
        await invoke('remove_from_vault', {
          vaultKeySignature,
          filePath: file.path || fileName,
          archiveAddress
        });

        toast.add({
          severity: 'success',
          summary: 'Removed from Vault',
          detail: `${isArchive ? 'Archive' : 'File'} "${fileName}" has been removed from your vault.`,
          life: 3000,
        });

        // Refresh the vault files
        fileStore.getAllFiles();

      } catch (innerError: any) {
        console.error('Failed to remove from vault:', innerError);
        toast.add({
          severity: 'error',
          summary: 'Removal Failed',
          detail: innerError?.message || 'Failed to remove from vault.',
          life: 3000,
        });
      } finally {
        // Always clear loading state
        pendingVaultRemoval.value = false;
        vaultRemovalItem.value = null;
      }
    },
    reject: () => {
      // No action needed, user cancelled
    }
  });
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
        group: 'download-success'
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

  // Removed legacy payment-request event listener - we only use upload-quote now

  // Set up upload quote event listener
  await listen("upload-quote", async (event: any) => {
    const payload = event.payload;
    console.log(">>> Upload quote received for ID:", payload.upload_id);

    // Find the upload by ID - much simpler since frontend and backend use same ID!
    const upload = uploadsStore.uploads.find(u => u.id === payload.upload_id);
    const isModalUpload = upload && modalUploadId.value === upload.id;

    if (!upload) {
      console.warn(">>> Upload not found for quote:", payload.upload_id);
      return;
    }

    // Store quote data for this specific upload
    const uploadQuoteData = {
      totalFiles: payload.total_files,
      totalSize: formatBytes(payload.total_size || 0),
      totalCostFormatted: payload.total_cost_formatted, // Map to expected field name
      totalCostNano: payload.total_cost_nano,
      paymentRequired: payload.payment_required,
      payments: payload.payments, // Include payments array for display
      rawPayments: payload.raw_payments, // Raw payment data for wallet transaction
    };

    // Store quote data per upload ID
    uploadQuotes.value.set(payload.upload_id, uploadQuoteData);
    console.log(">>> Stored quote data for upload:", payload.upload_id, uploadQuoteData);

    // Handle modal uploads (with UI)
    if (isModalUpload && showUploadModal.value) {
      // Set modal quote data from the stored quote
      quoteData.value = {
        ...uploadQuoteData,
        files: pendingUploadFiles.value?.files,
        archiveName: pendingUploadFiles.value?.files?.[0]?.name || "",
        vaultKeySignature: pendingUploadFiles.value?.vaultKeySignature,
        isFolder: pendingUploadFiles.value?.isFolder
      };

      // Update UI to show quote received and set payment step
      updateStepStatus('quoting', 'completed', 'Quote received');

      // Check if this is a duplicate upload (cost = 0) first, regardless of payment_required flag
      if (payload.total_cost_nano === '0' || payload.total_cost_nano === 0) {
        console.log(">>> Duplicate upload detected (cost = 0) - waiting for backend completion event");
        updateStepStatus('payment-request', 'completed', 'No payment required');

        // Don't update upload status - wait for backend Completed event
        // The backend will emit a Completed event for duplicate uploads

        // Update UI to show completion
        updateStepStatus('quoting', 'completed', 'Quote received');
        updateStepStatus('payment-request', 'completed', 'Already uploaded');
      }
      // Set payment step based on whether payment is required
      else if (payload.payment_required && payload.payments && payload.payments.length > 0) {
        updateStepStatus('payment-request', 'pending', 'Ready for payment...');

        // Keep upload status as 'quoting' - payment modal will handle the transition
        // Status will change to 'uploading' after payment confirmation
      } else {
        // Free upload - no payment needed
        updateStepStatus('payment-request', 'completed', 'No payment required');

        // Don't update upload status - wait for backend events
        // The backend will handle the upload and emit appropriate events
        if (upload) {
          console.log(">>> Free upload detected - waiting for backend to process");

          // Update UI to show no payment required
          updateStepStatus('quoting', 'completed', 'Quote received');
          updateStepStatus('payment-request', 'completed', 'No payment required');
        }
      }

      console.log(">>> Quote data set:", quoteData.value);
    } else if (upload) {
      // Handle non-modal uploads (background uploads)
      console.log(">>> Processing quote for non-modal upload:", upload.id);

      // Check if this is a duplicate upload (cost = 0) first, regardless of payment_required flag
      if (payload.total_cost_nano === '0' || payload.total_cost_nano === 0) {
        console.log(">>> Non-modal duplicate upload detected (cost = 0) - waiting for backend completion event");

        // Don't update upload status - wait for backend Completed event
        // The backend will emit a Completed event for duplicate uploads

        // Switch to uploads tab to show the upload
        activeTab.value = 2;

        console.log(">>> Non-modal duplicate upload - waiting for backend");
      }
      // Set payment step based on whether payment is required
      else if (payload.payment_required && payload.payments && payload.payments.length > 0) {
        console.log(">>> Non-modal upload requires payment - upload will wait for manual payment");
        // For now, non-modal uploads that require payment will wait
        // User will need to pay for them manually later
        // TODO: In the future, we could implement auto-payment or batch payment
      } else {
        // Free upload - wait for backend events
        console.log(">>> Non-modal free upload detected - waiting for backend to process");

        // Don't update upload status - wait for backend events
        // The backend will handle the upload and emit appropriate events

        // Switch to uploads tab to show the upload
        activeTab.value = 2;

        console.log(">>> Non-modal free upload - waiting for backend");
      }
    }
  });

  // Set up upload progress event listener
  await listen("upload-progress", (event: any) => {
    const payload = event.payload;
    console.log(">>> Upload progress event:", payload.type, payload);

    // Get the upload ID from the payload and find the corresponding upload
    console.log(">>> Upload progress - looking for upload_id:", payload.upload_id);
    console.log(">>> Current uploads in store:", uploadsStore.uploads.map(u => ({id: u.id, status: u.status})));
    const upload = payload.upload_id ? uploadsStore.uploads.find(u => u.id === payload.upload_id) : null;
    const isModalUpload = upload && modalUploadId.value === upload.id;
    console.log(">>> Found upload:", upload?.id, "isModalUpload:", isModalUpload);

    // Helper function to update the correct upload
    const updateUploadById = (updates: any) => {
      if (upload) {
        console.log(">>> updateUploadById - updating upload:", upload.id, "with:", updates);
        uploadsStore.updateUpload(upload.id, updates);
        console.log(">>> updateUploadById - upload after update:", uploadsStore.uploads.find(u => u.id === upload.id));
      } else if (payload.upload_id) {
        // Fallback: try to find by modalUploadId or create a placeholder
        console.warn(">>> updateUploadById - Upload not found for ID:", payload.upload_id);
        console.warn(">>> Available uploads:", uploadsStore.uploads.map(u => ({id: u.id, status: u.status})));
        console.warn(">>> modalUploadId:", modalUploadId.value);

        // If this is for the modal upload ID, try to update it directly
        if (modalUploadId.value === payload.upload_id) {
          console.log(">>> Using modalUploadId as fallback");
          uploadsStore.updateUpload(modalUploadId.value, updates);
        }
      }
    };

    switch (payload.type) {
      case "Started":
        // Only update the global upload store for the modal upload
        if (isModalUpload) {
          uploadStore.startUpload(payload.total_files || 0, payload.total_size || 0);
        }

        updateUploadById({
          // Don't change status - should already be 'uploading' after payment
          totalSize: payload.total_size || 0
        });

        if (showUploadModal.value && isModalUpload) {
          // Set quote data with real file info immediately
          quoteData.value = {
            totalFiles: payload.total_files,
            totalSize: formatBytes(payload.total_size || 0)
          };
          // Start with quoting step
          updateStepStatus('quoting', 'processing', `Preparing ${payload.total_files} file(s)...`);
        }
        break;

      case "Processing":
        // Only update global upload store for modal upload
        if (isModalUpload) {
          uploadStore.updateProcessing(
              payload.current_file || "",
              payload.files_processed || 0,
              payload.bytes_processed || 0
          );
        }

        // Update the specific upload
        const processProgress = payload.total_bytes > 0 ? Math.round((payload.bytes_processed / payload.total_bytes) * 100) : 0;
        updateUploadById({
          // Don't change status - should stay as 'uploading'
          currentFile: payload.current_file,
          filesProcessed: payload.files_processed || 0,
          bytesProcessed: payload.bytes_processed || 0,
          progress: processProgress
        });

        if (showUploadModal.value && isModalUpload) {
          updateStepStatus('quoting', 'processing', `Processing: ${payload.current_file}`, processProgress);
        }
        break;

      case "Encrypting":
        // Only update global upload store for modal upload
        if (isModalUpload) {
          uploadStore.updateEncrypting(payload.current_file || "");
        }

        // Update the specific upload
        updateUploadById({
          // Don't change status - should stay as 'uploading'
          currentFile: payload.current_file,
          filesProcessed: payload.files_processed || 0
        });
        if (showUploadModal.value && isModalUpload) {
          // Check if this is the last file
          const filesProcessed = payload.files_processed || 0;
          const totalFiles = payload.total_files || 0;
          const isLastFile = filesProcessed >= totalFiles && totalFiles > 0;

          if (isLastFile) {
            // After last file, backend still needs to prepare the archive
            updateStepStatus('quoting', 'processing', `Finalizing preparation...`);
          } else {
            updateStepStatus('quoting', 'processing', `Processing: ${payload.current_file} (${filesProcessed + 1}/${totalFiles})`);
          }
        }
        break;

      case "RequestingPayment":
        // Only update global upload store for modal upload
        if (isModalUpload) {
          uploadStore.updateRequestingPayment();
        }

        // Update the specific upload
        updateUploadById({
          status: 'quoting'
        });

        if (showUploadModal.value && isModalUpload) {
          updateStepStatus('quoting', 'processing', 'Getting storage quote...');
          // Note: Payment request will be shown when we get the payment-request event
        }
        break;

      case "Uploading":
        console.log(">>> Uploading event - modalUploadId:", modalUploadId.value, "upload from payload:", upload?.id);

        // Only update the global upload store for the modal upload
        if (isModalUpload) {
          uploadStore.updateUploading(payload.chunks_uploaded || 0, payload.total_chunks || 0);
        }

        // Use the helper function to update the correct upload
        const progress = payload.total_chunks > 0 ? Math.round((payload.chunks_uploaded / payload.total_chunks) * 100) : 0;
        updateUploadById({
          status: 'uploading',
          chunksUploaded: payload.chunks_uploaded || 0,
          totalChunks: payload.total_chunks || 0,
          progress
        });

        console.log(`>>> Updated upload with chunks: ${payload.chunks_uploaded}/${payload.total_chunks}, progress: ${progress}%`);

        // Only handle modal cleanup for the specific upload that the modal is open for
        if (showUploadModal.value && modalUploadId.value === upload?.id) {
          // If we reach uploading, the payment was approved
          updateStepStatus('quoting', 'completed', 'Quote received');
          updateStepStatus('payment-request', 'completed', 'Payment authorized');

          // Close modal and show progress table
          showUploadModal.value = false;
          // Switch to uploads tab to show the active upload
          activeTab.value = 2;
          // Clean up modal state but don't clear modalUploadId yet - needed for completion
          pendingUploadFiles.value = null;
          uploadSteps.value = [];
          currentUploadStep.value = '';
          uploadError.value = '';
          quoteData.value = null;
          // DON'T clear modalUploadId here - it will be cleared in Completed/Failed events
        }
        break;

      case "Completed":
        console.log(">>> Upload completed event received", payload);

        // Only update the global upload store for the modal upload
        if (isModalUpload) {
          uploadStore.completeUpload();
        }

        // Update the specific upload
        updateUploadById({
          status: 'completed',
          progress: 100,
          completedAt: new Date()
        });

        // Close modal only if this completed upload is the one the modal is open for
        if (showUploadModal.value && modalUploadId.value === upload?.id) {
          setTimeout(() => {
            showUploadModal.value = false;
            // Switch to uploads tab to show completed upload
            activeTab.value = 2;
          }, 1500);
        }

        // Clear modalUploadId if this was the modal upload
        if (isModalUpload && modalUploadId.value === upload?.id) {
          modalUploadId.value = null;
        }

        // Clean up stored quote data for completed upload
        if (upload?.id) {
          uploadQuotes.value.delete(upload.id);
          console.log(">>> Cleaned up quote data for completed upload:", upload.id);
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
        // Only update the global upload store for the modal upload
        if (isModalUpload) {
          uploadStore.failUpload(payload.error || "Unknown error");
        }

        // Update the specific upload
        updateUploadById({
          status: 'failed',
          error: payload.error || "Unknown error",
          completedAt: new Date()
        });

        // Clear modalUploadId if this was the modal upload
        if (isModalUpload && modalUploadId.value === upload?.id) {
          modalUploadId.value = null;
        }

        // Clean up stored quote data for failed upload
        if (upload?.id) {
          uploadQuotes.value.delete(upload.id);
          console.log(">>> Cleaned up quote data for failed upload:", upload.id);
        }

        if (showUploadModal.value && isModalUpload) {
          uploadError.value = payload.error || "Unknown error";
        }
        setTimeout(() => {
          uploadStore.resetUpload();
        }, 5000);
        break;

      case "Cancelled":
        uploadStore.failUpload("Upload cancelled");
        updateUploadById({
          status: 'failed',
          error: "Upload cancelled",
          completedAt: new Date()
        });

        if (isModalUpload) {
          modalUploadId.value = null;
          if (showUploadModal.value) {
            showUploadModal.value = false;
            handleCancelUploadModal();
          }
        }

        setTimeout(() => {
          if (isModalUpload) {
            uploadStore.resetUpload();
          }
        }, 1000);
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

// Removed auto-complete stuck uploads logic - backend should handle upload completion properly

// Watch for tab changes to load local files when needed
watch(activeTab, (newTab, oldTab) => {
  // Clear breadcrumbs when switching tabs to ensure proper isolation
  if (oldTab === 0) {
    // Reset vault navigation when leaving vault tab
    breadcrumbs.value = [];
    if (rootDirectory.value) {
      fileStore.changeDirectory(rootDirectory.value);
    }
  } else if (oldTab === 1) {
    // Reset local files navigation when leaving local files tab
    localBreadcrumbs.value = [];
    if (localRootDirectory.value) {
      localFilesStore.changeDirectory(localRootDirectory.value);
    }
  }

  if (newTab === 1 && !localRootDirectory.value) {
    loadLocalFiles();
  }
});

// Debug watcher for uploads store
watch(() => uploadsStore.uploads, (newUploads) => {
  console.log(">>> Uploads store changed - count:", newUploads.length);
  console.log(">>> Active uploads count:", uploadsStore.activeUploads.length);
  console.log(">>> Active uploads:", uploadsStore.activeUploads.map(u => ({id: u.id, status: u.status, name: u.name})));
}, {deep: true});

watch(() => uploadsStore.activeUploads.length, (newCount, oldCount) => {
  console.log(">>> Active uploads count changed from", oldCount, "to", newCount);
});

// Watch for vault removal state to show/hide loading notification
watch(pendingVaultRemoval, (isRemoving) => {
  if (isRemoving && vaultRemovalItem.value) {
    const item = vaultRemovalItem.value;
    const itemType = item.isArchive ? 'archive' : 'file';
    emit('show-notify', {
      notifyType: 'info',
      title: 'Removing from vault',
      details: `Removing ${itemType} "${item.name}" from vault..`,
      enabledCancel: false,
    });
  } else {
    emit('hide-notify');
  }
});

// Get current local directory files for display with search filtering (similar to vault files)
const localDirectoryFiles = computed(() => {
  try {
    if (!localCurrentDirectory.value) {
      return [];
    }

    const files = localCurrentDirectoryFiles.value || [];

    if (query.value) {
      return files.filter((file: any) =>
          file.name.toLowerCase().includes(query.value.toLowerCase()) &&
          file.name !== 'parents'
      );
    }

    return files.filter((file: any) => file.name !== 'parents');
  } catch (error) {
    return [];
  }
});

// Combine local files with loading and failed archive states (similar to vault files)
const combinedLocalFiles = computed(() => {
  const regularFiles = localDirectoryFiles.value || [];

  // Only show failed and loading archives in the root directory
  const isRootDirectory = localCurrentDirectory.value === localRootDirectory.value;

  const failedArchiveFiles = isRootDirectory ? localFailedArchives.value
      .filter(archive => !query.value || archive.name.toLowerCase().includes(query.value.toLowerCase()))
      .map(archive => ({
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

  const loadingArchiveFiles = isRootDirectory ? localLoadingArchives.value
      .filter(archive => !query.value || archive.name.toLowerCase().includes(query.value.toLowerCase()))
      .map(archive => ({
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

  return [...regularFiles, ...failedArchiveFiles, ...loadingArchiveFiles].sort((a, b) => {
    // Sort by name, putting directories first, then files
    if (a.is_directory !== b.is_directory) {
      return a.is_directory ? -1 : 1;
    }
    return a.name.localeCompare(b.name, undefined, {numeric: true, sensitivity: 'base'});
  });
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

  // Removed stuck upload auto-completion - let backend handle upload lifecycle properly
});

// Removed onUnmounted cleanup for stuck upload interval
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
              v-if="(activeTab === 0 && currentDirectory?.parent) || (activeTab === 1 && localCurrentDirectory?.parent)"
              class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
              @click="activeTab === 1 ? handleLocalGoBack(localCurrentDirectory.parent) : handleGoBack(currentDirectory.parent)"
          >
            <i class="pi pi-reply -scale-x-100 translate"/>
          </div>

          <div
              class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-blue-600 hover:bg-autonomi-blue-700 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
              @click="$event => { refUploadDropdown.toggle($event); }"
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
              Vault
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
                  <template v-if="(file.path || file.isArchive || file.is_failed_archive) && !file.is_loading_archive">
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
                  <template v-if="(file.path || file.isArchive || file.is_failed_archive) && !file.is_loading_archive">
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
                  <template v-if="(file.path || file.isArchive || file.is_failed_archive) && !file.is_loading_archive">
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
                  <template v-if="(file.path || file.isArchive || file.is_failed_archive) && !file.is_loading_archive">
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
        <TabPanel
            :header="uploadsStore.activeUploads.length > 0 ? `Uploads (${uploadsStore.activeUploads.length})` : 'Uploads'"
            :value="2">
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
                          'bg-yellow-500': upload.status === 'quoting',
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

                      <!-- Duration or completion message -->
                      <span class="text-sm text-gray-500 dark:text-gray-400">
                      <template v-if="upload.completionMessage">
                        {{ upload.completionMessage }}
                      </template>
                      <template v-else>
                        took {{ formatUploadDuration(upload.createdAt, upload.completedAt) }}
                      </template>
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
        <TabPanel
            :header="downloadsStore.activeDownloads.length > 0 ? `Downloads (${downloadsStore.activeDownloads.length})` : 'Downloads'"
            :value="3">
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
              <i :class="[item.icon, item.class]"/>
              <div :class="item.class">
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

        <!-- Show data address for public files -->
        <div
            v-if="selectedFileItem?.file_access?.Public || selectedFileItem?.access_data?.Public || (selectedFileItem?.address && (selectedFileItem?.type === 'public_file' || selectedFileItem?.type === 'public_archive'))"
            class="py-3">
          <div class="flex items-center gap-2">
            <span>Data Address</span>
            <i
                class="pi pi-clipboard text-xs cursor-pointer hover:text-autonomi-blue-500"
                @click="handleCopyDataAddress(selectedFileItem)"
                v-tooltip.top="'Copy address'"
            />
          </div>
          <div class="text-autonomi-text-primary font-mono text-xs break-all">
            <template v-if="selectedFileItem?.file_access?.Public">
              {{ selectedFileItem.file_access.Public }}
            </template>
            <template v-else-if="selectedFileItem?.access_data?.Public">
              {{ selectedFileItem.access_data.Public }}
            </template>
            <template v-else-if="selectedFileItem?.address">
              {{ selectedFileItem.address }}
            </template>
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

<style>
/* Make confirm dialog narrower */
:deep(.p-dialog.p-confirm-dialog) {
  max-width: 450px !important;
  width: 90% !important;
}

:deep(.p-confirm-dialog .p-dialog-content) {
  width: 100% !important;
}

:deep(.p-confirm-dialog .p-confirm-dialog-message) {
  margin: 1.5rem 0;
  word-wrap: break-word;
}
</style>