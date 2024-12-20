<script lang="ts" setup>
import {useToast} from "primevue/usetoast";
import {invoke} from "@tauri-apps/api/core";
import {open} from '@tauri-apps/plugin-dialog';
import {basename} from '@tauri-apps/api/path';
import {useWalletStore} from "~/stores/wallet";

const walletStore = useWalletStore();
const toast = useToast();

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
  const files = await Promise.all(selected.map(async (file) => {
    return {path: file, name: await basename(file)};
  }));

  let secretKey = await walletStore.getVaultKey();

  await invoke("upload_files", {files, secretKey});
};
</script>

<template>
  <div>
    <!-- UPLOADER PAGE -->
    <div
        class="autonomi-uploader px-[100px] py-[70px]"
    >
      <!-- <Toast /> -->

      <CommonButton variant="secondary" @click="openPickerAndUploadFiles">
        Upload Files
      </CommonButton>
    </div>
  </div>
</template>
