import {invoke} from "@tauri-apps/api/core";
import type {IFolder, IFile, IFailedArchive} from "~/types/folder";

interface ILocalArchive {
    name: string;
    address: string;
    is_private: boolean;
    files?: any[];
}

interface ILocalFileStructure {
    archives: ILocalArchive[];
    failed_archives: IFailedArchive[];
    files: any[];
}

export const useLocalFilesStore = defineStore("localFiles", () => {
    const toast = useToast();

    // Class - reuse the same Folder class from files store
    class Folder {
        name: string;
        paths: any;
        parent: any;
        children: any[] = [];
        isArchive: boolean = false;
        archive?: ILocalArchive;

        constructor(name: string, parent = null, paths = null, isArchive = false, archive?: ILocalArchive) {
            this.name = name;
            this.parent = parent;
            this.isArchive = isArchive;
            this.archive = archive;
        }

        // Add a subfolder
        addSubfolder(subfolder: IFolder) {
            try {
                if (this.children.find((child) => child.name === subfolder.name)) {
                    throw new Error("Subfolder already exists");
                }

                subfolder.parent = this;
                this.children.push(subfolder);
            } catch (error) {
                console.log(">>> ERROR: Failed to add local subfolder", error);
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

                file.parent = this;
                this.children.push(file);
            } catch (error) {
                console.log(">>> ERROR: Failed to add local file", error);
            }
        }

        getParent() {
            return this.parent;
        }

        getChild(name: string) {
            return this.children.find((child) => child.name === name);
        }
    }

    // State
    const localFiles = ref<IFile[]>([]);
    const localStructure = ref<ILocalFileStructure | null>(null);
    const failedArchives = ref<IFailedArchive[]>([]);
    const loadingArchives = ref<{name: string, address: string, is_private: boolean}[]>([]);
    const rootDirectory = ref<IFolder | null>(null);
    const currentDirectory = ref<IFolder | null>(null);
    const pendingLocalStructure = ref(false);
    const loadedArchives = ref<Map<string, any>>(new Map());
    const currentLoadCode = ref<string | null>(null);

    // Computed
    const currentDirectoryFiles = computed(() => {
        if (!currentDirectory.value) {
            return [];
        }
        return currentDirectory.value.children || [];
    });

    // Actions
    const buildRootDirectory = () => {
        try {
            rootDirectory.value = null;

            if (!localStructure.value?.archives.length && !localStructure.value?.files?.length) {
                return;
            }

            console.log(">>> Building Local Archive-based Drive...");
            rootDirectory.value = new Folder("Local Files");

            // Process archives that have been loaded
            localStructure.value.archives.forEach((archive: ILocalArchive, archiveIndex: number) => {
                // Only process archives that have files loaded
                if (!archive.files || archive.files.length === 0) {
                    return;
                }

                // Check if archive has a name (not empty after sanitization)
                const hasName = archive.name && archive.name.trim() !== '';
                
                if (!hasName) {
                    // Unnamed archive - add files directly to root
                    archive.files.forEach((file: any) => {
                        const fileParts = file.path.split("/").filter((part: string) => part.length > 0);
                        let current: any = rootDirectory.value;

                        fileParts.forEach((part: string, index: number) => {
                            if (index === fileParts.length - 1) {
                                // This is the file - add directly to current folder
                                current.addFile({
                                    path: file.path,
                                    metadata: file.metadata,
                                    file_access: file.file_access,
                                    access_data: file.file_access,
                                    is_loaded: true,
                                    is_loading: false,
                                    load_error: false,
                                    name: part,
                                    archive_name: archive.name || `archive_${archiveIndex}`,
                                    type: archive.is_private ? 'private_file' : 'public_file'
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
                    
                    // Handle duplicate archive names by checking if it's actually a different archive
                    while (rootDirectory.value!.getChild(archiveFolderName)) {
                        const existingChild = rootDirectory.value!.getChild(archiveFolderName);
                        // If it's the same archive (same address), don't create a duplicate
                        if (existingChild && existingChild.archive && existingChild.archive.address === archive.address) {
                            // Same archive, don't add counter - just skip creating a new folder
                            return;
                        }
                        archiveFolderName = `${archive.name} (${counter})`;
                        counter++;
                    }

                    const archiveFolder = new Folder(archiveFolderName, rootDirectory.value, null, true, archive);
                    rootDirectory.value!.addSubfolder(archiveFolder);

                    // Add files within the archive folder
                    archive.files.forEach((file: any) => {
                        const fileParts = file.path.split("/").filter((part: string) => part.length > 0);
                        let current: any = archiveFolder;

                        fileParts.forEach((part: string, index: number) => {
                            if (index === fileParts.length - 1) {
                                // This is the file
                                current.addFile({
                                    path: file.path,
                                    metadata: file.metadata,
                                    file_access: file.file_access,
                                    access_data: file.file_access,
                                    is_loaded: true,
                                    is_loading: false,
                                    load_error: false,
                                    name: part,
                                    archive_name: archive.name,
                                    type: archive.is_private ? 'private_file' : 'public_file'
                                });
                            } else {
                                // This is a subdirectory within the archive
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
            localStructure.value?.files?.forEach((file: any) => {
                const fileParts = file.name.split("/").filter((part: string) => part.length > 0);
                let current: any = rootDirectory.value;

                fileParts.forEach((part: string, index: number) => {
                    if (index === fileParts.length - 1) {
                        // This is the file
                        current.addFile({
                            path: file.name,
                            metadata: { size: 0, uploaded: 0 },
                            file_access: null,
                            access_data: null,
                            is_loaded: true,
                            is_loading: false,
                            load_error: false,
                            name: part,
                            archive_name: "",
                            type: file.type || 'unknown'
                        });
                    } else {
                        // This is a subdirectory
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
            console.log(">>> ERROR: Failed to build local archive-based drive", error);
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
            console.log(">>> ERROR: Failed to change local directory", error);
        }
    };

    // Generate a unique temp code for this load operation
    const generateTempCode = () => {
        return `local_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
    };

    const getLocalStructure = async () => {
        console.log(">>> Getting local file structure with streaming...");
        
        // Generate new temp code for this load operation
        const tempCode = generateTempCode();
        currentLoadCode.value = tempCode;
        console.log(">>> Generated temp code for local load:", tempCode);
        
        // Clear all state to show loading
        localStructure.value = null;
        localFiles.value = [];
        failedArchives.value = [];
        loadingArchives.value = [];
        rootDirectory.value = null;
        currentDirectory.value = null;
        pendingLocalStructure.value = true;

        try {
            // Initialize local structure 
            localStructure.value = {
                archives: [],
                failed_archives: [],
                files: []
            };

            // Start streaming local structure updates with temp code
            await invoke("get_local_structure_streaming", { tempCode });

        } catch (error: any) {
            console.log(">>> ERROR: Failed to get local structure:", error);
            const message = error?.message || "There was an error getting your local files.";

            toast.add({
                severity: "error",
                summary: "Failed to get local files",
                detail: message,
                life: 3000,
            });

            throw new Error("Failed to get local files");
        } finally {
            // Note: don't set pendingLocalStructure to false here, it will be set when streaming completes
        }
    };

    const loadLocalArchive = async (archive: ILocalArchive) => {
        try {
            // Add to loading list
            if (!loadingArchives.value.find(a => a.address === archive.address)) {
                loadingArchives.value.push({
                    name: archive.name,
                    address: archive.address,
                    is_private: archive.is_private
                });
            }

            let archiveContents;
            if (archive.is_private) {
                archiveContents = await invoke('load_local_private_archive', {
                    localAddr: archive.address
                });
            } else {
                archiveContents = await invoke('load_local_public_archive', {
                    addressHex: archive.address
                });
            }
            
            // Remove from loading list
            loadingArchives.value = loadingArchives.value.filter(a => a.address !== archive.address);
            
            // Add files to archive and add to loaded archives
            archive.files = archiveContents;
            loadedArchives.value.set(archive.address, archive);
            
            if (localStructure.value) {
                localStructure.value.archives.push(archive);
            }

            console.log(`>>> Successfully loaded local archive: ${archive.name}`);
            
        } catch (error: any) {
            console.error(`>>> Failed to load local archive: ${archive.name}`, error);
            
            // Remove from loading list
            loadingArchives.value = loadingArchives.value.filter(a => a.address !== archive.address);
            
            // Add to failed archives
            const failedArchive = {
                name: archive.name,
                address: archive.address,
                is_private: archive.is_private
            };
            
            if (!failedArchives.value.find(a => a.address === archive.address)) {
                failedArchives.value.push(failedArchive);
                if (localStructure.value) {
                    localStructure.value.failed_archives.push(failedArchive);
                }
            }
        }
    };

    // Handle local updates from streaming
    const handleLocalUpdate = (update: any) => {
        console.log(">>> Received local update:", update.update_type, update);
        
        // Validate temp code - ignore update if it doesn't match current load operation
        if (!update.temp_code || update.temp_code !== currentLoadCode.value) {
            console.log(">>> Ignoring local update - temp code mismatch:", update.temp_code, "vs", currentLoadCode.value);
            return;
        }
        
        if (!localStructure.value) {
            localStructure.value = {
                archives: [],
                failed_archives: [],
                files: []
            };
        }

        switch (update.update_type) {
            case "IndividualFiles":
                // Add individual files immediately
                localStructure.value.files = update.files;
                
                // Clear and rebuild flattened files array to avoid duplicates
                localFiles.value = [];
                update.files.forEach((file: any) => {
                    localFiles.value.push({
                        path: file.name,
                        metadata: { size: 0, uploaded: 0 },
                        file_access: null,
                        is_loaded: true,
                        is_loading: false,
                        load_error: false,
                        name: file.name,
                        type: file.is_private ? 'private_file' : 'public_file'
                    });
                });

                // Build initial directory structure with individual files
                buildRootDirectory();
                
                // Hide loading once we have some content
                if (update.files.length > 0) {
                    pendingLocalStructure.value = false;
                }
                break;

            case "ArchiveLoading":
                if (update.loading_archive) {
                    // Add to loading archives list, avoiding duplicates
                    const exists = loadingArchives.value.some(a => 
                        a.name === update.loading_archive!.name && a.address === update.loading_archive!.address);
                    if (!exists) {
                        loadingArchives.value.push({
                            name: update.loading_archive.name,
                            address: update.loading_archive.address,
                            is_private: update.loading_archive.is_private
                        });
                    }
                }
                break;

            case "ArchiveLoaded":
                if (update.archive) {
                    // Remove from loading list
                    loadingArchives.value = loadingArchives.value.filter(
                        a => a.address !== update.archive!.address
                    );
                    
                    // Add the archive with files, avoiding duplicates
                    const archiveWithFiles = {
                        ...update.archive,
                        files: update.archive.files
                    };
                    
                    // Check if archive already exists by address
                    const existingArchive = localStructure.value.archives.find(a => a.address === archiveWithFiles.address);
                    if (!existingArchive) {
                        localStructure.value.archives.push(archiveWithFiles);
                    }
                    
                    // Add archive to loaded archives map
                    loadedArchives.value.set(update.archive.address, archiveWithFiles);

                    // Rebuild directory structure to include new archive
                    buildRootDirectory();
                    
                    // Hide loading once we have some content
                    if (localFiles.value.length > 0 || localStructure.value.archives.length > 0) {
                        pendingLocalStructure.value = false;
                    }
                }
                break;

            case "ArchiveFailed":
                if (update.failed_archive) {
                    // Remove from loading list
                    loadingArchives.value = loadingArchives.value.filter(
                        a => a.address !== update.failed_archive!.address
                    );
                    
                    // Add to failed archives list, avoiding duplicates
                    const exists = failedArchives.value.some(a => 
                        a.name === update.failed_archive!.name && a.address === update.failed_archive!.address);
                    if (!exists) {
                        localStructure.value.failed_archives.push(update.failed_archive);
                        failedArchives.value.push({
                            name: update.failed_archive.name,
                            address: update.failed_archive.address,
                            is_private: update.failed_archive.is_private
                        });
                    }
                    
                    // Rebuild directory to show failed archives
                    buildRootDirectory();
                }
                break;

            case "Complete":
                // Clear any remaining loading archives and finish loading
                loadingArchives.value = [];
                pendingLocalStructure.value = false;
                console.log(">>> Local structure streaming completed");
                break;
        }
    };

    // Return
    return {
        localFiles,
        localStructure,
        failedArchives,
        loadingArchives,
        rootDirectory,
        currentDirectory,
        currentDirectoryFiles,
        pendingLocalStructure,
        loadedArchives,
        // Methods
        changeDirectory,
        getLocalStructure,
        loadLocalArchive,
        buildRootDirectory,
        handleLocalUpdate
    };
});