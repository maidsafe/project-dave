export interface UploadItem {
  id: string;
  name: string;
  totalFiles: number;
  totalSize: number;
  status: 'pending' | 'processing' | 'encrypting' | 'quoting' | 'payment' | 'uploading' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  currentFile?: string;
  filesProcessed: number;
  bytesProcessed: number;
  chunksUploaded?: number;
  totalChunks?: number;
  error?: string;
  createdAt: Date;
  completedAt?: Date;
}

export const useUploadsStore = defineStore('uploads', () => {
  const uploads = ref<UploadItem[]>([]);

  const sortedUploads = computed(() => {
    return uploads.value
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());
  });

  const activeUploads = computed(() => {
    return sortedUploads.value.filter(upload => 
      ['pending', 'processing', 'encrypting', 'quoting', 'payment', 'uploading'].includes(upload.status)
    );
  });

  const completedUploads = computed(() => {
    return sortedUploads.value.filter(upload => upload.status === 'completed');
  });

  const failedUploads = computed(() => {
    return sortedUploads.value.filter(upload => upload.status === 'failed');
  });

  const createUpload = (files: Array<{path: string, name: string}>) => {
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
      status: 'pending',
      progress: 0,
      filesProcessed: 0,
      bytesProcessed: 0,
      createdAt: new Date()
    };

    uploads.value.push(upload);
    return uploadId;
  };

  const updateUpload = (uploadId: string, updates: Partial<UploadItem>) => {
    const index = uploads.value.findIndex(upload => upload.id === uploadId);
    if (index !== -1) {
      const updated = { ...uploads.value[index], ...updates };
      console.log(`>>> Uploads store: updating upload ${uploadId}`, updates, 'new state:', updated);
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
    updateUpload(uploadId, {
      status: 'cancelled',
      completedAt: new Date()
    });
  };

  const retryUpload = (uploadId: string) => {
    updateUpload(uploadId, {
      status: 'pending',
      progress: 0,
      filesProcessed: 0,
      bytesProcessed: 0,
      chunksUploaded: 0,
      error: undefined,
      completedAt: undefined
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
    retryUpload,
    clearCompleted,
    clearFailed
  };
});