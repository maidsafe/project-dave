import { defineStore } from "pinia";

export interface UploadProgressState {
  isUploading: boolean;
  currentFile: string;
  filesProcessed: number;
  totalFiles: number;
  bytesProcessed: number;
  totalBytes: number;
  chunksUploaded: number;
  totalChunks: number;
  progressPercentage: number;
  statusMessage: string;
  error: string | null;
}

export const useUploadStore = defineStore("upload", () => {
  // State
  const uploadProgress = ref<UploadProgressState>({
    isUploading: false,
    currentFile: "",
    filesProcessed: 0,
    totalFiles: 0,
    bytesProcessed: 0,
    totalBytes: 0,
    chunksUploaded: 0,
    totalChunks: 0,
    progressPercentage: 0,
    statusMessage: "",
    error: null,
  });

  // Actions
  const startUpload = (totalFiles: number, totalSize: number) => {
    uploadProgress.value = {
      isUploading: true,
      currentFile: "",
      filesProcessed: 0,
      totalFiles,
      bytesProcessed: 0,
      totalBytes: totalSize,
      chunksUploaded: 0,
      totalChunks: 0,
      progressPercentage: 0,
      statusMessage: `Preparing to upload ${totalFiles} file(s)...`,
      error: null,
    };
  };

  const updateProcessing = (currentFile: string, filesProcessed: number, bytesProcessed: number) => {
    uploadProgress.value.currentFile = currentFile;
    uploadProgress.value.filesProcessed = filesProcessed;
    uploadProgress.value.bytesProcessed = bytesProcessed;
    uploadProgress.value.progressPercentage = Math.round(
      (bytesProcessed / uploadProgress.value.totalBytes) * 100
    );
    uploadProgress.value.statusMessage = `Processing: ${currentFile}`;
  };

  const updateEncrypting = (currentFile: string) => {
    uploadProgress.value.currentFile = currentFile;
    uploadProgress.value.statusMessage = `Encrypting: ${currentFile}`;
  };

  const updateRequestingPayment = () => {
    uploadProgress.value.statusMessage = "Requesting payment authorization...";
  };

  const updateUploading = (chunksUploaded: number, totalChunks: number) => {
    uploadProgress.value.chunksUploaded = chunksUploaded;
    uploadProgress.value.totalChunks = totalChunks;
    uploadProgress.value.progressPercentage = Math.round((chunksUploaded / totalChunks) * 100);
    uploadProgress.value.statusMessage = `Uploading to network: ${chunksUploaded}/${totalChunks} chunks`;
  };

  const completeUpload = () => {
    uploadProgress.value.isUploading = false;
    uploadProgress.value.progressPercentage = 100;
    uploadProgress.value.statusMessage = `Upload complete! ${uploadProgress.value.totalFiles} file(s) uploaded.`;
  };

  const failUpload = (error: string) => {
    uploadProgress.value.isUploading = false;
    uploadProgress.value.error = error;
    uploadProgress.value.statusMessage = `Upload failed: ${error}`;
  };

  const resetUpload = () => {
    uploadProgress.value = {
      isUploading: false,
      currentFile: "",
      filesProcessed: 0,
      totalFiles: 0,
      bytesProcessed: 0,
      totalBytes: 0,
      chunksUploaded: 0,
      totalChunks: 0,
      progressPercentage: 0,
      statusMessage: "",
      error: null,
    };
  };

  return {
    uploadProgress: readonly(uploadProgress),
    startUpload,
    updateProcessing,
    updateEncrypting,
    updateRequestingPayment,
    updateUploading,
    completeUpload,
    failUpload,
    resetUpload,
  };
});