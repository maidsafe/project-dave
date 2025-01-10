<script lang="ts" setup>
import { useToast } from "primevue/usetoast";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { basename } from "@tauri-apps/api/path";
import { useWalletStore } from "~/stores/wallet";

const walletStore = useWalletStore();
const toast = useToast();
const emit = defineEmits(["show-notify", "hide-notify"]);

const openPickerAndUploadFiles = async () => {
  // Open the file picker.
  let selected = await open({ multiple: true });

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
      return { path: file, name: await basename(file) };
    })
  );

  try {
    console.log(">>> UPLOD.VUE GETTING VAULT KEY SIGNATURE");
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
    const uploadResponse = await invoke("upload_files", {
      files,
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

      <CommonButton variant="secondary" @click="openPickerAndUploadFiles">
        Upload Files
      </CommonButton>
    </div>
  </div>
</template>
