export interface DownloadItem {
  id: string;
  fileName: string;
  filePath: string; // Path in vault
  status: 'pending' | 'loading' | 'downloading' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  fileSize?: number;
  downloadedBytes?: number;
  downloadPath?: string; // Local download path
  error?: string;
  createdAt: Date;
  completedAt?: Date;
}

export const useDownloadsStore = defineStore('downloads', () => {
  const downloads = ref<Map<string, DownloadItem>>(new Map());

  const sortedDownloads = computed(() => {
    return Array.from(downloads.value.values())
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());
  });

  const activeDownloads = computed(() => {
    return sortedDownloads.value.filter(download => 
      ['pending', 'loading', 'downloading'].includes(download.status)
    );
  });

  const completedDownloads = computed(() => {
    return sortedDownloads.value.filter(download => download.status === 'completed');
  });

  const failedDownloads = computed(() => {
    return sortedDownloads.value.filter(download => download.status === 'failed');
  });

  const createDownload = (file: any) => {
    const downloadId = Date.now().toString() + Math.random().toString(36).substr(2, 9);
    
    const download: DownloadItem = {
      id: downloadId,
      fileName: file.name || 'unknown_file',
      filePath: file.path,
      status: 'pending',
      progress: 0,
      fileSize: file.metadata?.size,
      createdAt: new Date()
    };

    downloads.value.set(downloadId, download);
    return downloadId;
  };

  const updateDownload = (downloadId: string, updates: Partial<DownloadItem>) => {
    const download = downloads.value.get(downloadId);
    if (download) {
      const updated = { ...download, ...updates };
      
      // Calculate progress if we have size info
      if (updated.fileSize && updated.downloadedBytes) {
        updated.progress = Math.round((updated.downloadedBytes / updated.fileSize) * 100);
      }
      
      downloads.value.set(downloadId, updated);
    }
  };

  const removeDownload = (downloadId: string) => {
    downloads.value.delete(downloadId);
  };

  const cancelDownload = (downloadId: string) => {
    updateDownload(downloadId, {
      status: 'cancelled',
      completedAt: new Date()
    });
  };

  const retryDownload = (downloadId: string) => {
    updateDownload(downloadId, {
      status: 'pending',
      progress: 0,
      downloadedBytes: 0,
      error: undefined,
      completedAt: undefined
    });
  };

  const clearCompleted = () => {
    const completedIds = Array.from(downloads.value.entries())
      .filter(([_, download]) => download.status === 'completed')
      .map(([id, _]) => id);
    
    completedIds.forEach(id => downloads.value.delete(id));
  };

  const clearFailed = () => {
    const failedIds = Array.from(downloads.value.entries())
      .filter(([_, download]) => download.status === 'failed')
      .map(([id, _]) => id);
    
    failedIds.forEach(id => downloads.value.delete(id));
  };

  return {
    downloads,
    sortedDownloads,
    activeDownloads,
    completedDownloads,
    failedDownloads,
    createDownload,
    updateDownload,
    removeDownload,
    cancelDownload,
    retryDownload,
    clearCompleted,
    clearFailed
  };
});