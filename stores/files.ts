import {invoke} from "@tauri-apps/api/core";
import type {IFolder, IFile} from "~/types/folder";
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
                const existingFile = this.children.find((child) => child.name === file.name);
                if (existingFile) {
                    // Update existing file if new one has more data
                    if (file.is_loaded && !existingFile.is_loaded) {
                        Object.assign(existingFile, file);
                    }
                    return;
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
    const vaultStructure = ref<any>(null);
    const failedArchives = ref<any[]>([]);
    const rootDirectory = ref<IFolder | null>(null);
    const currentDirectory = ref<IFolder | null>(null);
    const pendingFilesSignature = ref(false);
    const pendingVaultStructure = ref(false);
    const loadedFiles = ref<Map<string, any>>(new Map());

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

    const getVaultStructure = async () => {
        console.log(">>> Getting vault structure...");
        try {
            // Get vault key signature
            pendingFilesSignature.value = true;
            let vaultKeySignature = await walletStore.getVaultKeySignature();
            pendingFilesSignature.value = false;

            // Get vault structure (fast - just metadata)
            pendingVaultStructure.value = true;
            vaultStructure.value = await invoke("get_vault_structure", {vaultKeySignature});

            // Store failed archives
            failedArchives.value = vaultStructure.value.failed_archives || [];

            // Convert structure to files format for building directory
            files.value = vaultStructure.value.files.map((file: any) => ({
                path: file.path,
                metadata: file.metadata,
                file_access: file.file_type === "Private" ? {Private: null} : {Public: null},
                is_loaded: false,
                is_loading: false,
                load_error: false
            }));

            // Build Root Directory immediately
            buildRootDirectory();

        } catch (error: any) {
            console.log(">>> ERROR: Failed to get vault structure:", error);
            const message =
                error?.message || "There was an error getting your vault structure.";

            toast.add({
                severity: "error",
                summary: "Failed to get vault structure",
                detail: message,
                life: 3000,
            });

            throw new Error("Failed to get vault structure");
        } finally {
            pendingFilesSignature.value = false;
            pendingVaultStructure.value = false;
        }
    };


    const getAllFiles = async () => {
        // This is now just an alias for the new two-phase approach
        return getVaultStructure();
    };

    const loadSingleFileData = async (file: any) => {
        try {
            // Mark the file as loading
            const fileIndex = files.value.findIndex(f => f.path === file.path);
            if (fileIndex !== -1) {
                files.value[fileIndex] = {
                    ...files.value[fileIndex],
                    is_loading: true,
                    load_error: false
                };
            }

            // Get vault key signature
            let vaultKeySignature = await walletStore.getVaultKeySignature();
            
            // Load the file data
            const loadedFile = await invoke("get_single_file_data", {
                vaultKeySignature,
                filePath: file.path
            }) as any;

            // Update the file with loaded data
            if (fileIndex !== -1) {
                files.value[fileIndex] = {
                    ...loadedFile,
                    is_loaded: true,
                    is_loading: false,
                    load_error: false
                };

                // Store in loadedFiles map
                loadedFiles.value.set(loadedFile.path, loadedFile);
            }

            return loadedFile;
        } catch (error: any) {
            console.error("Failed to load file data:", error);
            
            // Mark the file as failed to load
            const fileIndex = files.value.findIndex(f => f.path === file.path);
            if (fileIndex !== -1) {
                files.value[fileIndex] = {
                    ...files.value[fileIndex],
                    is_loaded: false,
                    is_loading: false,
                    load_error: true
                };
            }
            
            throw error;
        }
    };
    
    // Return
    return {
        files,
        vaultStructure,
        failedArchives,
        rootDirectory,
        currentDirectory,
        currentDirectoryFiles,
        pendingFilesSignature,
        pendingVaultStructure,
        loadedFiles,
        // Methods
        changeDirectory,
        getAllFiles,
        getVaultStructure,
        loadSingleFileData,
    };
});
