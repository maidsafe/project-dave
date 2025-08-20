<script lang="ts" setup>
import {useToast} from "primevue/usetoast";
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-dialog";
import {basename} from "@tauri-apps/api/path";
import {useWalletStore} from "~/stores/wallet";
import {useUploadStore} from "~/stores/upload";

const walletStore = useWalletStore();
const uploadStore = useUploadStore();
const toast = useToast();
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

// Common function to upload files
const uploadFiles = async (files: Array<{ path: string, name: string }>) => {
  try {
    console.log(">>> UPLOAD.VUE GETTING VAULT KEY SIGNATURE");
    emit("show-notify", {
      notifyType: "info",
      title: "Sign upload required",
      details: "Please sign the upload request in your mobile wallet.",
    });

    let vaultKeySignature = await walletStore.getVaultKeySignature();

    // TODO: Show list of selected files
    // TODO: Allow user to select multiple files from different directories
    // TODO: Move invoke to a separate function. Give user the option to upload files???
    console.log(">>> UPLOAD.VUE INVOKING UPLOAD_FILES");
    emit("show-notify", {
      notifyType: "info",
      title: "Invoice",
      details: "Please wait - fetching invoice details.",
    });
    
    // Generate archive name
    const archiveName = files.length === 1 
      ? files[0].name  // Single file: use filename
      : `${files.length}_files_${Date.now()}`; // Multiple files: use count and timestamp

    const uploadResponse = await invoke("upload_files", {
      files,
      archiveName,
      vaultKeySignature,
    });
    console.log(">>> UPLOAD.VUE INVOKED UPLOAD_FILES COMPLETE");
  } catch (error: any) {
    toast.add({
      severity: "error",
      summary: "Error uploading files",
      detail: error.message,
      life: 3000,
    });
  } finally {
    emit("hide-notify");
  }
};
</script>

<template>
  <div>
    <!-- UPLOADER PAGE -->
    <div class="autonomi-uploader px-[100px] py-[70px]">
      <!-- <Toast /> -->

      <div class="flex gap-4">
        <CommonButton 
          variant="secondary" 
          @click="openPickerAndUploadFiles"
          :disabled="isUploading"
        >
          <span v-if="!isUploading">Upload Files</span>
          <span v-else>Uploading...</span>
        </CommonButton>
        <CommonButton 
          variant="secondary" 
          @click="openFolderPickerAndUploadFiles"
          :disabled="isUploading"
        >
          <span v-if="!isUploading">Upload Folder</span>
          <span v-else>Uploading...</span>
        </CommonButton>
      </div>
    </div>
  </div>
</template>
