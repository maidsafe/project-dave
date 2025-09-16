export interface UploadItem {
  id: string;
  // Removed backendUploadId - we now use single ID throughout
  name: string;
  totalFiles: number;
  totalSize: number;
  status: 'quoting' | 'uploading' | 'completed' | 'failed';
  progress: number;
  currentFile?: string;
  filesProcessed: number;
  bytesProcessed: number;
  chunksUploaded?: number;
  totalChunks?: number;
  error?: string;
  completionMessage?: string;
  createdAt: Date;
  completedAt?: Date;
  addToVault?: boolean;
}

// No longer need invoke since cancellation is frontend-only

export const useUploadsStore = defineStore('uploads', () => {
  const uploads = ref<UploadItem[]>([]);

  const sortedUploads = computed(() => {
    return uploads.value
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());
  });

  const activeUploads = computed(() => {
    return sortedUploads.value.filter(upload => 
      ['quoting', 'uploading'].includes(upload.status)
    );
  });

  const completedUploads = computed(() => {
    return sortedUploads.value.filter(upload => upload.status === 'completed');
  });

  const failedUploads = computed(() => {
    return sortedUploads.value.filter(upload => upload.status === 'failed');
  });

  const createUpload = (files: Array<{path: string, name: string}>, addToVault: boolean = true) => {
    const uploadId = Date.now().toString() + Math.random().toString(36).substr(2, 9);
    const totalFiles = files.length;
    
    // Calculate display name
    let name: string;
    if (totalFiles === 1) {
      name = files[0].name;
    } else {
      name = `${totalFiles} files`;
    }

    const upload: UploadItem = {
      id: uploadId,
      name,
      totalFiles,
      totalSize: 0, // Will be updated when we get the Started event
      status: 'quoting',
      progress: 0,
      filesProcessed: 0,
      bytesProcessed: 0,
      createdAt: new Date(),
      addToVault
    };

    uploads.value.push(upload);
    return uploadId;
  };

  const updateUpload = (uploadId: string, updates: Partial<UploadItem>) => {
    const index = uploads.value.findIndex(upload => upload.id === uploadId);
    if (index !== -1) {
      const updated = { ...uploads.value[index], ...updates };
      console.log(`>>> Uploads store: updating upload ${uploadId}`, updates, 'new state:', updated);
      
      // Add stack trace if status is being set to failed due to cancellation
      if (updates.status === 'failed' && updates.error?.includes('cancelled')) {
        console.trace('>>> Upload being cancelled - stack trace:');
      }
      
      uploads.value[index] = updated;
    } else {
      console.log(`>>> Uploads store: upload ${uploadId} not found when trying to update`);
    }
  };

  const removeUpload = (uploadId: string) => {
    const index = uploads.value.findIndex(upload => upload.id === uploadId);
    if (index !== -1) {
      uploads.value.splice(index, 1);
    }
  };

  const cancelUpload = (uploadId: string) => {
    // Uploads can only be cancelled before payment/execution phase
    // No backend call needed since upload hasn't started yet
    updateUpload(uploadId, {
      status: 'failed',
      error: 'Upload cancelled',
      completedAt: new Date()
    });
  };


  const clearCompleted = () => {
    uploads.value = uploads.value.filter(upload => upload.status !== 'completed');
  };

  const clearFailed = () => {
    uploads.value = uploads.value.filter(upload => upload.status !== 'failed');
  };

  return {
    uploads,
    sortedUploads,
    activeUploads,
    completedUploads,
    failedUploads,
    createUpload,
    updateUpload,
    removeUpload,
    cancelUpload,
    clearCompleted,
    clearFailed
  };
});