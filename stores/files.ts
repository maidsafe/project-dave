import { invoke } from "@tauri-apps/api/core";
import type { IFolder, IFile } from "~/types/folder";
import { useWalletStore } from "~/stores/wallet";

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
        const paths = file.path.split("/").filter((name) => !!name); // Removes empty file names
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
      // files.value = await invoke("get_files_from_vault", { vaultKeySignature });

      // Dummy Test Data
      files.value = JSON.parse(
        '[{"path":"/particle_playground.png","metadata":{"uploaded":1734897515,"created":1734897515,"modified":1734897515,"size":1034240},"file_access":{"Private":[129,165,70,105,114,115,116,147,148,0,220,0,32,204,240,204,222,204,189,204,192,204,179,97,92,204,133,204,174,204,249,2,2,204,133,7,204,174,65,204,151,105,204,180,204,215,204,233,204,207,89,0,100,204,228,204,214,204,234,42,117,86,119,220,0,32,33,204,240,204,149,92,61,204,183,116,204,213,204,220,204,170,31,204,128,204,169,29,204,242,124,204,161,41,121,13,204,238,36,204,217,204,135,204,187,97,204,170,204,188,93,204,182,20,115,206,0,5,66,170,148,1,220,0,32,204,156,204,132,67,74,94,35,204,234,41,28,204,183,204,135,11,204,128,120,204,166,19,204,231,204,221,204,155,204,255,204,187,204,204,34,85,6,84,67,204,175,204,136,204,136,19,43,220,0,32,85,204,252,204,223,14,204,177,8,11,73,93,66,204,134,204,207,204,176,55,23,26,58,58,204,136,204,175,204,252,13,88,55,121,204,218,204,142,204,155,112,204,128,122,204,196,206,0,5,66,170,148,2,220,0,32,204,186,204,238,204,168,63,204,250,204,203,204,224,204,173,204,167,15,57,22,204,169,106,31,204,189,7,204,172,97,204,197,88,42,10,20,204,191,34,85,120,204,223,204,139,16,204,209,220,0,32,104,204,178,204,138,204,136,18,204,164,63,204,221,204,212,3,6,204,207,204,169,204,180,74,204,254,20,87,204,248,93,67,3,115,204,163,101,204,140,114,204,241,204,211,204,231,204,253,204,212,206,0,5,66,172]}},{"path":"/particledfdsf_playground22.png","metadata":{"uploaded":1734897515,"created":1734897515,"modified":1734897515,"size":1034240},"file_access":{"Private":[129,165,70,105,114,115,116,147,148,0,220,0,32,204,240,204,222,204,189,204,192,204,179,97,92,204,133,204,174,204,249,2,2,204,133,7,204,174,65,204,151,105,204,180,204,215,204,233,204,207,89,0,100,204,228,204,214,204,234,42,117,86,119,220,0,32,33,204,240,204,149,92,61,204,183,116,204,213,204,220,204,170,31,204,128,204,169,29,204,242,124,204,161,41,121,13,204,238,36,204,217,204,135,204,187,97,204,170,204,188,93,204,182,20,115,206,0,5,66,170,148,1,220,0,32,204,156,204,132,67,74,94,35,204,234,41,28,204,183,204,135,11,204,128,120,204,166,19,204,231,204,221,204,155,204,255,204,187,204,204,34,85,6,84,67,204,175,204,136,204,136,19,43,220,0,32,85,204,252,204,223,14,204,177,8,11,73,93,66,204,134,204,207,204,176,55,23,26,58,58,204,136,204,175,204,252,13,88,55,121,204,218,204,142,204,155,112,204,128,122,204,196,206,0,5,66,170,148,2,220,0,32,204,186,204,238,204,168,63,204,250,204,203,204,224,204,173,204,167,15,57,22,204,169,106,31,204,189,7,204,172,97,204,197,88,42,10,20,204,191,34,85,120,204,223,204,139,16,204,209,220,0,32,104,204,178,204,138,204,136,18,204,164,63,204,221,204,212,3,6,204,207,204,169,204,180,74,204,254,20,87,204,248,93,67,3,115,204,163,101,204,140,114,204,241,204,211,204,231,204,253,204,212,206,0,5,66,172]}},{"path":"/particle_pldfdsyground33.png","metadata":{"uploaded":1734897515,"created":1734897515,"modified":1734897515,"size":1034240},"file_access":{"Private":[129,165,70,105,114,115,116,147,148,0,220,0,32,204,240,204,222,204,189,204,192,204,179,97,92,204,133,204,174,204,249,2,2,204,133,7,204,174,65,204,151,105,204,180,204,215,204,233,204,207,89,0,100,204,228,204,214,204,234,42,117,86,119,220,0,32,33,204,240,204,149,92,61,204,183,116,204,213,204,220,204,170,31,204,128,204,169,29,204,242,124,204,161,41,121,13,204,238,36,204,217,204,135,204,187,97,204,170,204,188,93,204,182,20,115,206,0,5,66,170,148,1,220,0,32,204,156,204,132,67,74,94,35,204,234,41,28,204,183,204,135,11,204,128,120,204,166,19,204,231,204,221,204,155,204,255,204,187,204,204,34,85,6,84,67,204,175,204,136,204,136,19,43,220,0,32,85,204,252,204,223,14,204,177,8,11,73,93,66,204,134,204,207,204,176,55,23,26,58,58,204,136,204,175,204,252,13,88,55,121,204,218,204,142,204,155,112,204,128,122,204,196,206,0,5,66,170,148,2,220,0,32,204,186,204,238,204,168,63,204,250,204,203,204,224,204,173,204,167,15,57,22,204,169,106,31,204,189,7,204,172,97,204,197,88,42,10,20,204,191,34,85,120,204,223,204,139,16,204,209,220,0,32,104,204,178,204,138,204,136,18,204,164,63,204,221,204,212,3,6,204,207,204,169,204,180,74,204,254,20,87,204,248,93,67,3,115,204,163,101,204,140,114,204,241,204,211,204,231,204,253,204,212,206,0,5,66,172]}},{"path":"/particle_playground44.png","metadata":{"uploaded":1734897515,"created":1734897515,"modified":1734897515,"size":1034240},"file_access":{"Private":[129,165,70,105,114,115,116,147,148,0,220,0,32,204,240,204,222,204,189,204,192,204,179,97,92,204,133,204,174,204,249,2,2,204,133,7,204,174,65,204,151,105,204,180,204,215,204,233,204,207,89,0,100,204,228,204,214,204,234,42,117,86,119,220,0,32,33,204,240,204,149,92,61,204,183,116,204,213,204,220,204,170,31,204,128,204,169,29,204,242,124,204,161,41,121,13,204,238,36,204,217,204,135,204,187,97,204,170,204,188,93,204,182,20,115,206,0,5,66,170,148,1,220,0,32,204,156,204,132,67,74,94,35,204,234,41,28,204,183,204,135,11,204,128,120,204,166,19,204,231,204,221,204,155,204,255,204,187,204,204,34,85,6,84,67,204,175,204,136,204,136,19,43,220,0,32,85,204,252,204,223,14,204,177,8,11,73,93,66,204,134,204,207,204,176,55,23,26,58,58,204,136,204,175,204,252,13,88,55,121,204,218,204,142,204,155,112,204,128,122,204,196,206,0,5,66,170,148,2,220,0,32,204,186,204,238,204,168,63,204,250,204,203,204,224,204,173,204,167,15,57,22,204,169,106,31,204,189,7,204,172,97,204,197,88,42,10,20,204,191,34,85,120,204,223,204,139,16,204,209,220,0,32,104,204,178,204,138,204,136,18,204,164,63,204,221,204,212,3,6,204,207,204,169,204,180,74,204,254,20,87,204,248,93,67,3,115,204,163,101,204,140,114,204,241,204,211,204,231,204,253,204,212,206,0,5,66,172]}},{"path":"/particle_play3332232ground.png","metadata":{"uploaded":1734897515,"created":1734897515,"modified":1734897515,"size":1034240},"file_access":{"Private":[129,165,70,105,114,115,116,147,148,0,220,0,32,204,240,204,222,204,189,204,192,204,179,97,92,204,133,204,174,204,249,2,2,204,133,7,204,174,65,204,151,105,204,180,204,215,204,233,204,207,89,0,100,204,228,204,214,204,234,42,117,86,119,220,0,32,33,204,240,204,149,92,61,204,183,116,204,213,204,220,204,170,31,204,128,204,169,29,204,242,124,204,161,41,121,13,204,238,36,204,217,204,135,204,187,97,204,170,204,188,93,204,182,20,115,206,0,5,66,170,148,1,220,0,32,204,156,204,132,67,74,94,35,204,234,41,28,204,183,204,135,11,204,128,120,204,166,19,204,231,204,221,204,155,204,255,204,187,204,204,34,85,6,84,67,204,175,204,136,204,136,19,43,220,0,32,85,204,252,204,223,14,204,177,8,11,73,93,66,204,134,204,207,204,176,55,23,26,58,58,204,136,204,175,204,252,13,88,55,121,204,218,204,142,204,155,112,204,128,122,204,196,206,0,5,66,170,148,2,220,0,32,204,186,204,238,204,168,63,204,250,204,203,204,224,204,173,204,167,15,57,22,204,169,106,31,204,189,7,204,172,97,204,197,88,42,10,20,204,191,34,85,120,204,223,204,139,16,204,209,220,0,32,104,204,178,204,138,204,136,18,204,164,63,204,221,204,212,3,6,204,207,204,169,204,180,74,204,254,20,87,204,248,93,67,3,115,204,163,101,204,140,114,204,241,204,211,204,231,204,253,204,212,206,0,5,66,172]}}]'
      );
    } catch (error: any) {
      // TODO: Handle error
      console.log(">>> ERROR: Failed to get files:", error);
      const message =
        error?.message || "There was an error getting your files.";

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
