import { invoke } from '@tauri-apps/api/core';
import type { IFolder, IFile } from "~/types/folder";
import {useWalletStore} from "~/stores/wallet";

export const useFileStore = defineStore("files", () => {
  const walletStore = useWalletStore();
  // const autonomi = useAutonomiStore();
  const toast = useToast();

  // Class
  class Folder {
    name: string;
    paths: any;
    parent: any;
    children: any[] = [];

    constructor(name: string, parent = null, paths = null) {
      this.name = name;
      this.parent = parent;
      // this.paths = paths;
    }

    // Add asubfolder
    addSubfolder(subfolder: IFolder) {
      try {
        if (this.children.find((child) => child.name === subfolder.name)) {
          throw new Error("Subfolder already exists");
        }

        // Create subfolder
        subfolder.parent = this;
        this.children.push(subfolder);
      } catch (error) {
        console.log(">>> ERROR: Failed to add subfolder", error);
        // TODO: Message error creating sub folder
      }
    }

    // Add file
    addFile(file: any) {
      try {
        if (this.children.find((child) => child.name === file.name)) {
          throw new Error("File already exists");
        }

        // Create file
        file.parent = this;
        this.children.push(file);
      } catch (error) {
        console.log(">>> ERROR: Failed to add file", error);
      }
    }

    // Go up to parent folder
    getParent() {
      return this.parent;
    }

    // Gets child (subfolder or file) by name
    getChild(name: string) {
      return this.children.find((child) => child.name === name);
    }
  }

  // State
  const files = ref<IFile[]>([]);
  const rootDirectory = ref<IFolder | null>(null);
  const currentDirectory = ref<IFolder | null>(null);
  const pendingFilesSignature = ref(false);
  const pendingGetAllFiles = ref(false);

  // Computed
  const currentDirectoryFiles = computed(() => {
    if (!currentDirectory.value) {
      return [];
    }

    return Object.keys(currentDirectory.value);
  });

  // Actions
  const buildRootDirectory = () => {
    try {
      // Reset rootDirectory
      rootDirectory.value = null;

      if (!files.value.length) {
        return;
      }

      console.log(">>> Building Local Drive...");
      rootDirectory.value = new Folder("Root");

      files.value.forEach((file) => {
        // TODO: Change Parents to a name that will not be used as a folder name
        const paths = file.path.split("/");
        let current: any = rootDirectory.value;

        paths.forEach((path: string, index: number) => {
          if (index === paths.length - 1) {
            // current[path] = file;
            current.addFile({
              ...file,
              name: path,
            });
          } else {
            // Add subfolder
            let newFolder = current.getChild(path);
            if (!newFolder) {
              newFolder = new Folder(path);
              current.addSubfolder(newFolder);
            }

            // Update current folder
            current = newFolder;
          }
        });
      });

      // Set current directory
      currentDirectory.value = rootDirectory.value;
    } catch (error) {
      console.log(">>> ERROR: Failed to build local drive", error);
      rootDirectory.value = null;
    }
  };

  const changeDirectory = (directory: Folder) => {
    try {
      if (directory?.paths) {
        return;
      }

      currentDirectory.value = directory;
    } catch (error) {
      console.log(">>> ERROR: Failed to change directory", error);
    }
  };

  const getAllFiles = async () => {
    console.log(">>> Getting files from vault...");
    try {
      // Get vault key signature
      pendingFilesSignature.value = true;
      let vaultKeySignature = await walletStore.getVaultKeySignature();

      // Got signature
      pendingFilesSignature.value = false;

      // Set loading files
      pendingGetAllFiles.value = true;
      files.value = await invoke("get_files_from_vault", { vaultKeySignature });
    } catch (error: any) {
      // TODO: Handle error
      console.log(">>> ERROR: Failed to get files:", error);
      const message = error?.message || "There was an error getting your files.";

      toast.add({
        severity: "error",
        summary: "Failed to get files",
        detail: message,
        life: 3000,
      });

      throw new Error("Failed to get files");
    } finally {
      pendingFilesSignature.value = false;
      pendingGetAllFiles.value = false;

      // Build Root Directory
      buildRootDirectory();
    }
  };
  // Return
  return {
    files,
    rootDirectory,
    currentDirectory,
    currentDirectoryFiles,
    pendingFilesSignature,
    pendingGetAllFiles,
    // Methods
    changeDirectory,
    getAllFiles,
  };
});
