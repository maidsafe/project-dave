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
  const uploads = ref<Map<string, UploadItem>>(new Map());

  const sortedUploads = computed(() => {
    return Array.from(uploads.value.values())
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

    uploads.value.set(uploadId, upload);
    return uploadId;
  };

  const updateUpload = (uploadId: string, updates: Partial<UploadItem>) => {
    const upload = uploads.value.get(uploadId);
    if (upload) {
      const updated = { ...upload, ...updates };
      uploads.value.set(uploadId, updated);
    }
  };

  const removeUpload = (uploadId: string) => {
    uploads.value.delete(uploadId);
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
    const completedIds = Array.from(uploads.value.entries())
      .filter(([_, upload]) => upload.status === 'completed')
      .map(([id, _]) => id);
    
    completedIds.forEach(id => uploads.value.delete(id));
  };

  const clearFailed = () => {
    const failedIds = Array.from(uploads.value.entries())
      .filter(([_, upload]) => upload.status === 'failed')
      .map(([id, _]) => id);
    
    failedIds.forEach(id => uploads.value.delete(id));
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