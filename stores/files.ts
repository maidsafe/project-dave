import {invoke} from "@tauri-apps/api/core";
import type {IFolder, IFile, IVaultStructure, IArchive, IFailedArchive, IFileMetadata} from "~/types/folder";
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
        isArchive: boolean = false;
        archive?: IArchive;

        constructor(name: string, parent = null, paths = null, isArchive = false, archive?: IArchive) {
            this.name = name;
            this.parent = parent;
            this.isArchive = isArchive;
            this.archive = archive;
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
    const vaultStructure = ref<IVaultStructure | null>(null);
    const failedArchives = ref<IFailedArchive[]>([]);
    const loadingArchives = ref<{name: string, is_private: boolean}[]>([]);
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

            if (!vaultStructure.value?.archives.length && !vaultStructure.value?.files?.length) {
                return;
            }

            console.log(">>> Building Archive-based Local Drive...");
            rootDirectory.value = new Folder("Root");

            vaultStructure.value.archives.forEach((archive: IArchive, archiveIndex: number) => {
                // Check if archive has a name (not empty after sanitization)
                const hasName = archive.name && archive.name.trim() !== '';
                
                if (!hasName) {
                    // Unnamed archive - add files directly to root
                    archive.files.forEach((file: IFileMetadata) => {
                        const fileParts = file.path.split("/").filter(part => part.length > 0);
                        let current: any = rootDirectory.value;

                        fileParts.forEach((part: string, index: number) => {
                            if (index === fileParts.length - 1) {
                                // This is the file - add directly to current folder
                                current.addFile({
                                    path: file.path,
                                    metadata: file.metadata,
                                    file_access: file.access_data,
                                    access_data: file.access_data,
                                    is_loaded: !!file.access_data,
                                    is_loading: false,
                                    load_error: false,
                                    name: part,
                                    archive_name: archive.name || `archive_${archiveIndex}`
                                });
                            } else {
                                // This is a subdirectory - create regular folder (not archive folder)
                                let subFolder = current.getChild(part);
                                if (!subFolder) {
                                    subFolder = new Folder(part, current);
                                    current.addSubfolder(subFolder);
                                }
                                current = subFolder;
                            }
                        });
                    });
                } else {
                    // Named archive - create archive folder with unique name if needed
                    let archiveFolderName = archive.name;
                    let counter = 1;
                    
                    // Handle duplicate archive names by appending a counter
                    while (rootDirectory.value!.getChild(archiveFolderName)) {
                        archiveFolderName = `${archive.name} (${counter})`;
                        counter++;
                    }

                    const archiveFolder = new Folder(archiveFolderName, rootDirectory.value, null, true, archive);
                    rootDirectory.value!.addSubfolder(archiveFolder);

                    // Add files within the archive folder
                    archive.files.forEach((file: IFileMetadata) => {
                        const fileParts = file.path.split("/").filter(part => part.length > 0);
                        let current: any = archiveFolder;

                        fileParts.forEach((part: string, index: number) => {
                            if (index === fileParts.length - 1) {
                                // This is the file
                                current.addFile({
                                    path: file.path,
                                    metadata: file.metadata,
                                    file_access: file.access_data,
                                    access_data: file.access_data,
                                    is_loaded: !!file.access_data,
                                    is_loading: false,
                                    load_error: false,
                                    name: part,
                                    archive_name: archive.name
                                });
                            } else {
                                // This is a subdirectory within the archive
                                // Allow duplicate directory names within different archives by not checking globally
                                let subFolder = current.getChild(part);
                                if (!subFolder) {
                                    subFolder = new Folder(part, current);
                                    current.addSubfolder(subFolder);
                                }
                                current = subFolder;
                            }
                        });
                    });
                }
            });

            // Process individual files (not in archives)
            vaultStructure.value?.files?.forEach((file: IFileMetadata) => {
                const fileParts = file.path.split("/").filter(part => part.length > 0);
                let current: any = rootDirectory.value;

                fileParts.forEach((part: string, index: number) => {
                    if (index === fileParts.length - 1) {
                        // This is the file - add directly to current folder
                        current.addFile({
                            path: file.path,
                            metadata: file.metadata,
                            file_access: file.access_data,
                            access_data: file.access_data,
                            is_loaded: file.is_loaded,
                            is_loading: false,
                            load_error: false,
                            name: part,
                            archive_name: "" // Individual files have no archive
                        });
                    } else {
                        // This is a subdirectory - create regular folder (not archive folder)
                        let subFolder = current.getChild(part);
                        if (!subFolder) {
                            subFolder = new Folder(part, current);
                            current.addSubfolder(subFolder);
                        }
                        current = subFolder;
                    }
                });
            });

            // Set current directory
            currentDirectory.value = rootDirectory.value;
        } catch (error) {
            console.log(">>> ERROR: Failed to build archive-based local drive", error);
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
        console.log(">>> Getting vault structure with streaming...");
        
        // IMMEDIATELY clear all state to show loading
        vaultStructure.value = null;
        files.value = [];
        failedArchives.value = [];
        loadingArchives.value = [];
        rootDirectory.value = null;
        currentDirectory.value = null;
        pendingVaultStructure.value = true;

        try {
            // Get vault key signature
            pendingFilesSignature.value = true;
            let vaultKeySignature = await walletStore.getVaultKeySignature();
            pendingFilesSignature.value = false;

            // Initialize vault structure 
            vaultStructure.value = {
                archives: [],
                failed_archives: [],
                files: []
            };

            // Start streaming vault structure updates
            await invoke("get_vault_structure_streaming", {vaultKeySignature});

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
            // Note: don't set pendingVaultStructure to false here, it will be set when streaming completes
        }
    };

    // Handle vault updates from streaming
    const handleVaultUpdate = (update: any) => {
        console.log(">>> Received vault update:", update.update_type, update);
        
        if (!vaultStructure.value) {
            vaultStructure.value = {
                archives: [],
                failed_archives: [],
                files: []
            };
        }

        switch (update.update_type) {
            case "IndividualFiles":
                // Add individual files immediately
                vaultStructure.value.files = update.files;
                
                // Update flattened files array
                update.files.forEach((file: IFileMetadata) => {
                    files.value.push({
                        path: file.path,
                        metadata: file.metadata,
                        file_access: file.file_type === "Private" ? {Private: null} : {Public: null},
                        is_loaded: file.is_loaded,
                        is_loading: false,
                        load_error: false
                    });
                });

                // Build initial directory structure with individual files
                buildRootDirectory();
                
                // Hide loading once we have some content
                if (update.files.length > 0) {
                    pendingVaultStructure.value = false;
                }
                break;

            case "ArchiveLoading":
                if (update.loading_archive) {
                    // Add to loading archives list
                    loadingArchives.value.push(update.loading_archive);
                }
                break;

            case "ArchiveLoaded":
                if (update.archive) {
                    // Remove from loading list
                    loadingArchives.value = loadingArchives.value.filter(
                        a => a.name !== update.archive!.name
                    );
                    
                    // Add the archive
                    vaultStructure.value.archives.push(update.archive);
                    
                    // Add archive files to flattened array
                    update.archive.files.forEach((file: IFileMetadata) => {
                        files.value.push({
                            path: file.path,
                            metadata: file.metadata,
                            file_access: file.file_type === "Private" ? {Private: null} : {Public: null},
                            is_loaded: false,
                            is_loading: false,
                            load_error: false
                        });
                    });

                    // Rebuild directory structure to include new archive
                    buildRootDirectory();
                    
                    // Hide loading once we have some content
                    if (files.value.length > 0) {
                        pendingVaultStructure.value = false;
                    }
                }
                break;

            case "ArchiveFailed":
                if (update.failed_archive) {
                    // Remove from loading list
                    loadingArchives.value = loadingArchives.value.filter(
                        a => a.name !== update.failed_archive!.name
                    );
                    
                    vaultStructure.value.failed_archives.push(update.failed_archive);
                    failedArchives.value.push(update.failed_archive);
                    
                    // Rebuild directory to show failed archives
                    buildRootDirectory();
                }
                break;

            case "Complete":
                // Clear any remaining loading archives and finish loading
                loadingArchives.value = [];
                pendingVaultStructure.value = false;
                console.log(">>> Vault structure streaming completed");
                break;
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
        loadingArchives,
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
        handleVaultUpdate,
    };
});
