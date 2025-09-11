import {invoke} from "@tauri-apps/api/core";
import type {IFolder, IFile, IFailedArchive} from "~/types/folder";

interface ILocalArchive {
    name: string;
    archive_access: {
        Private: string;
    } | {
        Public: string;
    };
    files?: any[];
}

interface ILocalFileStructure {
    archives: ILocalArchive[];
    failed_archives: IFailedArchive[];
    files: any[];
}

// Helper function to extract address from FileAccess structure
function extractAddressFromFileAccess(fileAccess: any): string | null {
    if (fileAccess?.Public) {
        return fileAccess.Public;
    } else if (fileAccess?.Private) {
        return fileAccess.Private;
    }
    return null;
}

export const useLocalFilesStore = defineStore("localFiles", () => {
    const toast = useToast();
    
    // Helper functions for archive_access
    const getArchiveAddress = (archive: ILocalArchive): string => {
        if ('Private' in archive.archive_access) {
            return archive.archive_access.Private;
        }
        return archive.archive_access.Public;
    };
    
    const isPrivateArchive = (archive: ILocalArchive): boolean => {
        return 'Private' in archive.archive_access;
    };

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
    const loadingArchives = ref<{ name: string, file_access: any, is_private: boolean }[]>([]);
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
            rootDirectory.value = new Folder("Local Vault");

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
                                    archive_name: archive.name,
                                    archive_access: archive.archive_access,
                                    type: isPrivateArchive(archive) ? 'private_file' : 'public_file'
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
                        if (existingChild && existingChild.archive && getArchiveAddress(existingChild.archive) === getArchiveAddress(archive)) {
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
                                    archive_access: archive.archive_access,
                                    type: isPrivateArchive(archive) ? 'private_file' : 'public_file'
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
                            metadata: {size: 0, uploaded: 0},
                            file_access: file.file_access,
                            access_data: file.file_access,
                            is_loaded: true,
                            is_loading: false,
                            load_error: false,
                            name: part,
                            archive_name: "",
                            type: file.is_private ? 'private_file' : 'public_file'
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
            await invoke("get_local_structure_streaming", {tempCode});

        } catch (error: any) {
            console.log(">>> ERROR: Failed to get local structure:", error);
            const message = error?.message || "There was an error getting your local vault.";

            toast.add({
                severity: "error",
                summary: "Failed to get local vault",
                detail: message,
                life: 3000,
            });

            throw new Error("Failed to get local vault");
        } finally {
            // Note: don't set pendingLocalStructure to false here, it will be set when streaming completes
        }
    };

    const loadLocalArchive = async (archive: ILocalArchive) => {
        try {
            const archiveAddress = getArchiveAddress(archive);
            // Add to loading list
            if (!loadingArchives.value.find(a => extractAddressFromFileAccess(a.file_access) === archiveAddress)) {
                loadingArchives.value.push({
                    name: archive.name,
                    file_access: archive.archive_access,
                    is_private: isPrivateArchive(archive)
                });
            }

            let archiveContents;
            if (isPrivateArchive(archive)) {
                archiveContents = await invoke('load_local_private_archive', {
                    localAddr: archiveAddress
                });
            } else {
                archiveContents = await invoke('load_local_public_archive', {
                    addressHex: archiveAddress
                });
            }

            // Remove from loading list
            loadingArchives.value = loadingArchives.value.filter(a => extractAddressFromFileAccess(a.file_access) !== archiveAddress);

            // Add files to archive and add to loaded archives
            archive.files = archiveContents;
            loadedArchives.value.set(archiveAddress, archive);

            if (localStructure.value) {
                localStructure.value.archives.push(archive);
            }

            console.log(`>>> Successfully loaded local archive: ${archive.name}`);

        } catch (error: any) {
            console.error(`>>> Failed to load local archive: ${archive.name}`, error);

            const archiveAddress = getArchiveAddress(archive);
            // Remove from loading list
            loadingArchives.value = loadingArchives.value.filter(a => extractAddressFromFileAccess(a.file_access) !== archiveAddress);

            // Add to failed archives
            const failedArchive = {
                name: archive.name,
                address: archiveAddress,
                is_private: isPrivateArchive(archive)
            };

            if (!failedArchives.value.find(a => a.address === archiveAddress)) {
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
                        metadata: {size: 0, uploaded: 0, created: 0, modified: 0},
                        file_access: file.file_access,
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
                    const loadingArchiveAddress = extractAddressFromFileAccess(update.loading_archive.file_access);
                    const exists = loadingArchives.value.some(a =>
                        a.name === update.loading_archive!.name && extractAddressFromFileAccess(a.file_access) === loadingArchiveAddress);
                    if (!exists) {
                        loadingArchives.value.push({
                            name: update.loading_archive.name,
                            file_access: update.loading_archive.file_access,
                            is_private: update.loading_archive.is_private
                        });
                    }
                }
                break;

            case "ArchiveLoaded":
                if (update.archive) {
                    // Remove from loading list and get archive address
                    const archiveAddress = extractAddressFromFileAccess(update.archive.file_access);
                    loadingArchives.value = loadingArchives.value.filter(
                        a => extractAddressFromFileAccess(a.file_access) !== archiveAddress
                    );

                    // Transform the archive to use archive_access structure
                    const transformedArchive: ILocalArchive = {
                        name: update.archive.name,
                        archive_access: update.archive.is_private 
                            ? { Private: archiveAddress }
                            : { Public: archiveAddress },
                        files: update.archive.files
                    };

                    // Check if archive already exists by address
                    const existingArchive = localStructure.value.archives.find(a => 
                        getArchiveAddress(a) === archiveAddress
                    );
                    if (!existingArchive) {
                        localStructure.value.archives.push(transformedArchive);
                    }

                    // Add archive to loaded archives map
                    loadedArchives.value.set(archiveAddress, transformedArchive);

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
                    const failedArchiveAddress = extractAddressFromFileAccess(update.failed_archive.file_access);
                    loadingArchives.value = loadingArchives.value.filter(
                        a => extractAddressFromFileAccess(a.file_access) !== failedArchiveAddress
                    );

                    // Add to failed archives list, avoiding duplicates
                    const exists = failedArchives.value.some(a =>
                        a.name === update.failed_archive!.name && a.address === failedArchiveAddress);
                    if (!exists) {
                        const failedArchive = {
                            name: update.failed_archive.name,
                            address: failedArchiveAddress,
                            is_private: update.failed_archive.is_private
                        };
                        localStructure.value.failed_archives.push(failedArchive);
                        failedArchives.value.push(failedArchive);
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