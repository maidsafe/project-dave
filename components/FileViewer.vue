<script lang="ts" setup>
import { useFileStore } from "~/stores/files";
import { useToast } from "primevue/usetoast";
import { useUserStore } from "~/stores/user";

const toast = useToast();
const fileStore = useFileStore();
const {
  pendingGetAllFiles,
  currentDirectory,
  currentDirectoryFiles,
  rootDirectory,
} = storeToRefs(fileStore);
const userStore = useUserStore();
// const autonomi = useAutonomiStore();
const { query } = storeToRefs(userStore);
const view = ref<"vault">("vault");
const viewTypeVault = ref<"grid" | "list">("list");
const breadcrumbs = ref<any[]>([]);
const isVisibleFileInfo = ref(false);
const refFilesMenu = ref();
const refFilesViewMenu = ref();
const refDownloadMenu = ref();
const refUploadMenu = ref();
const selectedDownloadItem = ref<any>();
const selectedFileItem = ref<any>();
const selectedUploadItem = ref<any>();

const filteredFiles = computed(() => {
  try {
    if (!currentDirectory.value?.children?.length) {
      return [];
    }

    return currentDirectory.value.children.filter((folder: any) => {
      if (query.value) {
        // TODO: Change "parents" folder name
        return (
          folder.name.toLowerCase().includes(query.value.toLowerCase()) &&
          folder.name !== "parents"
        );
      }

      return folder.name !== "parents";
    });
  } catch (error) {
    // TODO: Handle error
    return [];
  }
});

const handleGoBack = (target: any) => {
  // Update breadcrumbs
  breadcrumbs.value.pop();

  // This is a folder, change directory
  fileStore.changeDirectory(target);
};

const handleChangeDirectory = (target: any) => {
  if (target?.paths) {
    // This is a file
    // toast.add({
    //   severity: "info",
    //   summary: "File",
    //   detail: "TODO: Handle Click on File",
    //   life: 6000,
    // });
    return;
  } else {
    // Update breadcrumbs
    breadcrumbs.value.push(target);
    // This is a folder, change directory
    fileStore.changeDirectory(target);
  }
};

const handleClickBreadcrumb = (crumb: any) => {
  // Remove all breadcrumbs after the clicked one
  const index = breadcrumbs.value.findIndex(
    (breadcrumb) => breadcrumb === crumb
  );

  breadcrumbs.value = breadcrumbs.value.slice(0, index + 1);

  fileStore.changeDirectory(crumb);
};

const handleStartUpload = () => {
  toast.add({
    severity: "info",
    summary: "Upload",
    detail: "TODO: Handle Start Upload",
    life: 6000,
  });
};

const handlePauseUpload = () => {
  toast.add({
    severity: "info",
    summary: "Upload",
    detail: "TODO: Handle Pause Upload",
    life: 6000,
  });
};

const handleCancelUpload = () => {
  toast.add({
    severity: "info",
    summary: "Upload",
    detail: "TODO: Handle Cancel Upload",
    life: 6000,
  });
};

const handleStartDownload = () => {
  toast.add({
    severity: "info",
    summary: "Download",
    detail: "TODO: Handle Start Download",
    life: 6000,
  });
};

const handlePauseDownload = () => {
  toast.add({
    severity: "info",
    summary: "Download",
    detail: "TODO: Handle Pause Download",
    life: 6000,
  });
};

const handleCancelDownload = () => {
  toast.add({
    severity: "info",
    summary: "Download",
    detail: "TODO: Handle Cancel Download",
    life: 6000,
  });
};

const menuUploads = ref([
  {
    label: "Start",
    icon: "pi pi-check",
    command: handleStartUpload,
  },
  {
    label: "Pause",
    icon: "pi pi-pause",
    command: handlePauseUpload,
  },
  {
    label: "Cancel",
    icon: "pi pi-times",
    command: handleCancelUpload,
  },
]);

const menuDownloads = ref([
  {
    label: "Start",
    icon: "pi pi-check",
    command: handleStartDownload,
  },
  {
    label: "Pause",
    icon: "pi pi-pause",
    command: handlePauseDownload,
  },
  {
    label: "Cancel",
    icon: "pi pi-times",
    command: handleCancelDownload,
  },
]);

const handleToggleUploadMenu = (event: any) => {
  refUploadMenu.value.toggle(event);
};

const handleToggleDownloadMenu = (event: any) => {
  refUploadMenu.value.toggle(event);
};

const handleToggleFileMenu = (event: any) => {
  refFilesMenu.value.toggle(event);
};

const handleRenameFile = () => {
  toast.add({
    severity: "info",
    summary: "File",
    detail: "TODO: Handle Rename File",
    life: 6000,
  });
};

const handleMoveFile = () => {
  toast.add({
    severity: "info",
    summary: "File",
    detail: "TODO: Handle Move File",
    life: 6000,
  });
};

const handleDownloadFile = async () => {
  const file = selectedFileItem.value;
  let fileBytes = new Uint8Array();

  if (file.privateDataAccess) {
    fileBytes = await autonomi.getPrivateData(file.privateDataAccess);
  } else if (file.dataMapAddress) {
    fileBytes = await autonomi.getData(file.dataMapAddress);
  } else {
    // TODO: return error
    return;
  }

  // Create a Blob from the bytes
  const blob = new Blob([fileBytes], { type: "application/octet-stream" });

  // Create a URL for the Blob
  const url = URL.createObjectURL(blob);

  // Create an <a> element and set the download attribute
  const a = document.createElement("a");
  a.href = url;
  a.download = file.name;

  // Trigger the download by clicking the <a> element
  document.body.appendChild(a); // Append <a> to the DOM
  a.click();
  document.body.removeChild(a); // Clean up

  // Release the URL after the download
  URL.revokeObjectURL(url);
};

const handleDeleteFile = () => {
  toast.add({
    severity: "info",
    summary: "File",
    detail: "TODO: Handle Delete File",
    life: 6000,
  });
};

const handleInfoFile = () => {
  isVisibleFileInfo.value = true;
};

const menuFiles = ref([
  {
    label: "Download",
    icon: "pi pi-download",
    command: handleDownloadFile,
  },
  // {
  //   label: "Rename",
  //   icon: "pi pi-pencil",
  //   command: handleRenameFile,
  // },
  // {
  //   label: "Delete",
  //   icon: "pi pi-trash",
  //   command: handleDeleteFile,
  // },
  {
    label: "Info",
    icon: "pi pi-info-circle",
    command: handleInfoFile,
  },
]);

const handleShowListView = () => {
  viewTypeVault.value = "list";
};

const handleShowGridView = () => {
  viewTypeVault.value = "grid";
};

const menuFilesView = ref([
  {
    label: "List",
    icon: "pi pi-list",
    command: handleShowListView,
  },
  {
    label: "Grid",
    icon: "pi pi-th-large",
    command: handleShowGridView,
  },
]);

const handleToggleFilesViewMenu = (event: any) => {
  refFilesViewMenu.value.toggle(event);
};

const secondsToDate = (seconds: number): Date => {
  return new Date(seconds * 1000);
};

onMounted(() => {
  try {
    // TODO: Check user / wallet permissions and details
    fileStore.getAllFiles();

    console.log(">>> Local Drive: ", rootDirectory);
    console.log(">>> Current directory: ", currentDirectory);
    console.log(">>> Current directory files: ", currentDirectoryFiles);

    console.log(">>> Filtered files: ", filteredFiles);
  } catch (err) {
    // TODO: Handle error
  }
});
</script>

<template>
  <div class="pr-[66px] pl-[66px] lg:pl-[110px] mt-10">
    <!-- View Toggler -->
    <div
      v-if="view === 'vault'"
      class="flex items-center justify-end gap-3 -mr-[30px] lg:-mr-0"
    >
      <div
        v-if="currentDirectory?.parent"
        class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
        @click="handleGoBack(currentDirectory.parent)"
      >
        <i class="pi pi-reply -scale-x-100 translate" />
      </div>

      <div
        class="w-10 h-10 rounded-full text-white flex items-center justify-center bg-autonomi-gray-600 hover:bg-autonomi-gray-600/70 cursor-pointer relative top-0 hover:-top-1 transition-all duration-300"
        @click="
          ($event) => {
            handleToggleFilesViewMenu($event);
          }
        "
      >
        <i class="pi pi-bars" />
      </div>
    </div>

    <!-- Breadcrumbs -->
    <div
      class="flex gap-4 items-center text-sm font-semibold flex-wrap mt-4 -ml-[30px] lg:-ml-0"
      v-if="breadcrumbs?.length > 0 && view === 'vault'"
    >
      <div
        class="cursor-pointer transition-all duration-300 text-autonomi-text-secondary"
        @click="handleClickBreadcrumb(rootDirectory)"
      >
        Root
      </div>
      <i class="text-xs pi pi-arrow-right text-autonomi-text-primary/70" />

      <template v-for="(crumb, index) in breadcrumbs" :key="index">
        <div
          :class="`cursor-pointer transition-all duration-300 ${
            index === breadcrumbs.length - 1
              ? 'text-autonomi-text-secondary'
              : 'text-autonomi-text-primary/70'
          }`"
          @click="handleClickBreadcrumb(crumb)"
        >
          {{ crumb.name }}
        </div>
        <i
          v-if="index !== breadcrumbs.length - 1"
          class="text-xs pi pi-arrow-right text-autonomi-text-primary/70"
        />
      </template>
    </div>

    <!-- View Navigation -->
    <div class="mt-[62px] -ml-[30px] -mr-[30px] lg:-ml-0 lg:-mr-0">
      <div class="grid grid-cols-12 h-10">
        <div
          class="col-span-12 lg:col-span-12 xl:col-span-12 flex flex-col justify-between"
        >
          <div
            :class="`text-sm font-semibold cursor-pointer transition-all duration-300 ${
              view === 'vault'
                ? 'text-autonomi-text-secondary'
                : 'text-autonomi-text-primary/70'
            }`"
            @click="view = 'vault'"
          >
            Files
          </div>
          <div
            :class="`h-1 transition-all duration-300 ${
              view === 'vault' ? 'bg-autonomi-blue-600' : 'bg-autonomi-blue-200'
            }`"
          />
        </div>
        <!--        <div-->
        <!--          class="col-span-4 lg:col-span-3 xl:col-span-2 flex flex-col justify-between"-->
        <!--        >-->
        <!--          <div-->
        <!--            :class="`text-sm font-semibold pl-3 lg:pl-12 cursor-pointer transition-all duration-300 ${-->
        <!--              view === 'uploads'-->
        <!--                ? 'text-autonomi-text-secondary'-->
        <!--                : 'text-autonomi-text-primary/70'-->
        <!--            }`"-->
        <!--            @click="view = 'uploads'"-->
        <!--          >-->
        <!--            Uploads-->
        <!--          </div>-->
        <!--          <div-->
        <!--            :class="`h-1 transition-all duration-300 ${-->
        <!--              view === 'uploads'-->
        <!--                ? 'bg-autonomi-blue-600'-->
        <!--                : 'bg-autonomi-blue-200'-->
        <!--            }`"-->
        <!--          />-->
        <!--        </div>-->
        <!--        <div-->
        <!--          class="col-span-4 lg:col-span-3 xl:col-span-2 flex flex-col justify-between"-->
        <!--        >-->
        <!--          <div-->
        <!--            :class="`text-sm font-semibold pl-2 lg:pl-12 cursor-pointer transition-all duration-300 ${-->
        <!--              view === 'downloads'-->
        <!--                ? 'text-autonomi-text-secondary'-->
        <!--                : 'text-autonomi-text-primary/70'-->
        <!--            }`"-->
        <!--            @click="view = 'downloads'"-->
        <!--          >-->
        <!--            Downloads-->
        <!--          </div>-->
        <!--          <div-->
        <!--            :class="`h-1 transition-all duration-300 ${-->
        <!--              view === 'downloads'-->
        <!--                ? 'bg-autonomi-blue-600'-->
        <!--                : 'bg-autonomi-blue-200'-->
        <!--            }`"-->
        <!--          />-->
        <!--        </div>-->
        <!--        <div class="col-span-6 hidden xl:flex flex-col justify-between">-->
        <!--          <div class="flex gap-7 self-end">-->
        <!--            <div class="flex items-center gap-2">-->
        <!--              <div class="h-2 w-2 rounded-full bg-autonomi-red-300" />-->
        <!--              <div class="text-sm font-semibold text-autonomi-text-secondary">-->
        <!--                Completed-->
        <!--              </div>-->
        <!--            </div>-->

        <!--            <div class="flex items-center gap-2">-->
        <!--              <div class="h-2 w-2 rounded-full bg-autonomi-blue-600" />-->
        <!--              <div class="text-sm font-semibold text-autonomi-text-secondary">-->
        <!--                Paused-->
        <!--              </div>-->
        <!--            </div>-->

        <!--            <div class="flex items-center gap-2">-->
        <!--              <div class="h-2 w-2 rounded-full bg-autonomi-gray-300" />-->
        <!--              <div class="text-sm font-semibold text-autonomi-text-secondary">-->
        <!--                Not Started-->
        <!--              </div>-->
        <!--            </div>-->
        <!--          </div>-->
        <!--          <div class="h-1 bg-autonomi-blue-200" />-->
        <!--        </div>-->
        <!--        <div-->
        <!--          class="hidden col-span-3 lg:flex flex-col justify-between xl:hidden"-->
        <!--        >-->
        <!--          <div />-->
        <!--          <div class="h-1 bg-autonomi-blue-200" />-->
        <!--        </div>-->
      </div>

      <!--      &lt;!&ndash; MOBILE LEGEND &ndash;&gt;-->
      <!--      <div class="flex gap-7 justify-end xl:hidden mt-6">-->
      <!--        <div class="flex items-center gap-2">-->
      <!--          <div class="h-2 w-2 rounded-full bg-autonomi-red-300" />-->
      <!--          <div class="text-sm font-semibold text-autonomi-text-secondary">-->
      <!--            Completed-->
      <!--          </div>-->
      <!--        </div>-->

      <!--        <div class="flex items-center gap-2">-->
      <!--          <div class="h-2 w-2 rounded-full bg-autonomi-blue-600" />-->
      <!--          <div class="text-sm font-semibold text-autonomi-text-secondary">-->
      <!--            Paused-->
      <!--          </div>-->
      <!--        </div>-->

      <!--        <div class="flex items-center gap-2">-->
      <!--          <div class="h-2 w-2 rounded-full bg-autonomi-gray-300" />-->
      <!--          <div class="text-sm font-semibold text-autonomi-text-secondary">-->
      <!--            Not Started-->
      <!--          </div>-->
      <!--        </div>-->
      <!--      </div>-->
    </div>

    <!-- Files Views -->
    <div class="mt-11 -mr-[66px] -ml-[110px]">
      <!-- Viewing: Vault (LIST) -->
      <div
        v-if="view === 'vault' && viewTypeVault === 'list'"
        class="grid grid-cols-12 font-semibold mb-10"
      >
        <div
          class="col-span-11 md:col-span-9 pl-[80px] lg:pl-[110px] text-autonomi-red-300"
        >
          Name
        </div>
        <div class="hidden xl:block xl:col-span-2 text-autonomi-red-300">
          Upload Date
        </div>
        <div class="col-span-1 text-autonomi-red-300">
          <i class="pi pi-user" />
        </div>

        <!-- Spacer -->
        <div class="col-span-12 h-10" />

        <!-- Vault Files Rows -->
        <template v-if="filteredFiles.length">
          <div
            v-for="file in filteredFiles"
            class="grid grid-cols-subgrid col-span-12 h-11 items-center odd:bg-autonomi-gray-100 hover:bg-white"
            @click="handleChangeDirectory(file)"
            :class="{ 'cursor-pointer': !file.paths }"
          >
            <!-- Folder/File Name -->
            <div
              class="col-span-11 md:col-span-9 pl-[80px] lg:pl-[110px] flex items-center"
            >
              <template v-if="file?.paths">
                <!-- This is the file -->
                <i
                  v-if="/\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(file.name)"
                  class="pi pi-image mr-4"
                />
                <i
                  v-if="/\.(pdf)$/i.test(file.name)"
                  class="pi pi-file-pdf mr-4"
                />
                <i v-if="/\.(zip)$/i.test(file.name)" class="pi pi-box mr-4" />

                <span class="text-ellipsis overflow-hidden">{{
                  file.name
                }}</span>
              </template>
              <template v-else>
                <!-- This is the folder -->
                <i class="pi pi-folder mr-4" /><span
                  class="line-clamp-one text-ellipsis"
                  >{{ file.name }}</span
                >
              </template>
            </div>

            <!-- Upload Date -->
            <div
              class="hidden xl:block xl:col-span-2 text-autonomi-text-primary"
            >
              {{
                file.dateUploaded
                  ? secondsToDate(file.dateUploaded).toLocaleString()
                  : ""
              }}
            </div>

            <!-- Menu -->
            <template v-if="file.paths">
              <div class="col-span-1">
                <i
                  class="pi pi-ellipsis-v cursor-pointer hover:text-autonomi-gray-600"
                  @click="
                    ($event) => {
                      // TODO: Update key:values to match api
                      selectedFileItem = file;
                      handleToggleFileMenu($event);
                    }
                  "
                />
              </div>
            </template>
          </div>
        </template>
        <template v-else>
          <div
            class="grid grid-cols-subgrid col-span-12 items-center justify-center min-h-[100px] font-semibold text-4xl text-autonomi-blue-600/50"
          >
            <div v-if="pendingGetAllFiles" class="col-span-12 pl-[150px]">
              <i class="pi pi-spinner pi-spin mr-4" />Loading files...
            </div>
            <div v-else class="col-span-12 pl-[150px]">No files found.</div>
          </div>
        </template>
      </div>

      <!-- Viewing: Vault (GRID) -->
      <div
        v-if="view === 'vault' && viewTypeVault === 'grid'"
        class="grid grid-cols-12 font-semibold mb-10"
      >
        <div class="col-span-12 pl-[80px] lg:pl-[110px] text-autonomi-red-300">
          Name
        </div>

        <!-- Spacer -->
        <div class="col-span-12 h-10" />

        <!-- Vault Files Rows -->
        <div
          class="col-span-12 grid grid-cols-12 ml-[80px] lg:ml-[110px] mr-[30px] lg:mr-[66px] gap-2"
        >
          <template v-if="filteredFiles.length">
            <div
              v-for="file in filteredFiles"
              class="col-span-6 md:col-span-4 xl:col-span-3 aspect-square max-h-[200px] text-autonomi-text-primary hover:bg-white rounded-lg hover:text-autonomi-text-secondary transition-all duration-500"
              :class="{ 'cursor-pointer': !file.paths }"
              @click="handleChangeDirectory(file)"
            >
              <div
                class="flex flex-col items-center justify-center w-full h-full p-2"
              >
                <!-- Menu -->
                <template v-if="file.paths">
                  <div class="self-end">
                    <i
                      class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                      @click="
                        ($event) => {
                          // TODO: Update key:values to match api
                          selectedFileItem = file;
                          handleToggleFileMenu($event);
                        }
                      "
                    />
                  </div>
                </template>

                <!-- Folder/File Name -->
                <div
                  class="flex flex-col flex-1 items-center justify-center gap-3"
                >
                  <template v-if="file?.paths">
                    <!-- This is the file -->
                    <i
                      v-if="
                        /\.(png|jpg|jpeg|gif|bmp|webp|svg)$/i.test(file.name)
                      "
                      class="pi pi-image text-4xl"
                    />
                    <i
                      v-if="/\.(pdf)$/i.test(file.name)"
                      class="pi pi-file-pdf mr-4"
                    />
                    <i
                      v-if="/\.(zip)$/i.test(file.name)"
                      class="pi pi-box mr-4"
                    />

                    <span class="text-ellipsis overflow-hidden">{{
                      file.name
                    }}</span>
                  </template>
                  <template v-else>
                    <!-- This is the folder -->
                    <i class="pi pi-folder text-4xl lg:text-6xl" />
                    <div class="text-ellipsis overflow-hidden">
                      {{ file.name }}
                    </div>
                  </template>
                </div>
              </div>
            </div>
          </template>
          <template v-else>
            <div
              class="grid grid-cols-subgrid col-span-12 items-center justify-center min-h-[100px] font-semibold text-4xl text-autonomi-blue-600/50"
            >
              <div v-if="pendingGetAllFiles" class="col-span-12 pl-[150px]">
                <i class="pi pi-spinner pi-spin mr-4" />Loading files...
              </div>
              <div v-else class="col-span-12 pl-[150px]">No files found.</div>
            </div>
          </template>
        </div>
      </div>

      <!-- Viewing: Uploads -->
      <div
        v-if="view === 'uploads'"
        class="mr-[33px] lg:mr-[66px] ml-[80px] lg:ml-[110px] font-semibold mb-10 flex flex-col gap-4"
      >
        <template v-for="item in [1, 2, 3]">
          <div class="flex items-center gap-4">
            <!-- Row Details -->
            <div class="grid grid-cols-12 gap-y-2 flex-1">
              <div class="col-span-12 lg:col-span-3 flex items-center">
                <div>MyRandomFileName.zip</div>
                <div class="ml-auto lg:hidden">
                  <i
                    class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                    @click="
                      ($event) => {
                        selectedUploadItem = item;
                        handleToggleUploadMenu($event);
                      }
                    "
                  />
                </div>
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 20).toFixed(2) }} GB
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 12).toFixed(0) }} of 265
              </div>
              <div class="hidden lg:block col-span-3 whitespace-nowrap">
                <span class="text-autonomi-red-300"
                  >{{ (Math.random() * 100).toFixed(2) }}%</span
                >
                complete
              </div>
              <div class="col-span-12 flex gap-[2px]">
                <div
                  v-for="item in Array(100)"
                  class="h-[22px] flex-1"
                  :class="`${
                    Math.random() > 0.5
                      ? 'bg-autonomi-red-300'
                      : Math.random() > 0.5
                      ? 'bg-autonomi-gray-500'
                      : 'bg-autonomi-blue-600'
                  }`"
                ></div>
              </div>
              <div
                class="col-span-12 lg:hidden text-xs font-semibold text-autonomi-text-primary flex justify-between gap-2"
              >
                <div>14.2.GB</div>
                <div>€43.25</div>
                <div>12 of 265</div>
                <div>2h 56m</div>
              </div>
            </div>
            <!-- Row Menu -->
            <div class="self-end hidden lg:block">
              <i
                class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                @click="
                  ($event) => {
                    selectedUploadItem = item;
                    handleToggleUploadMenu($event);
                  }
                "
              />
            </div>
          </div>
        </template>
      </div>

      <!-- Viewing: Downloads -->
      <div
        v-if="view === 'downloads'"
        class="mr-[30px] lg:mr-[66px] ml-[80px] lg:ml-[110px] font-semibold mb-10 flex flex-col gap-4"
      >
        <template v-for="item in Array(5)">
          <div class="flex items-center gap-4">
            <!-- Row Details -->
            <div class="grid grid-cols-12 gap-y-2 flex-1">
              <div class="col-span-12 lg:col-span-6 flex items-center">
                <div>MyRandomFileName.zip</div>
                <div class="ml-auto lg:hidden">
                  <i
                    class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                    @click="
                      ($event) => {
                        selectedDownloadItem = item;
                        handleToggleDownloadMenu($event);
                      }
                    "
                  />
                </div>
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 20).toFixed(2) }} GB
              </div>
              <div class="hidden lg:block col-span-2">
                {{ (Math.random() * 12).toFixed(0) }} of 265
              </div>
              <div
                class="hidden lg:block col-span-2 text-right whitespace-nowrap"
              >
                <span class="text-autonomi-red-300"
                  >{{ (Math.random() * 100).toFixed(2) }}%</span
                >
                complete
              </div>
              <div class="col-span-12 flex gap-[2px] lg:mb-6">
                <div
                  v-for="item in Array(100)"
                  class="h-[22px] flex-1"
                  :class="`${
                    Math.random() > 0.5
                      ? 'bg-autonomi-red-300'
                      : Math.random() > 0.5
                      ? 'bg-autonomi-gray-500'
                      : 'bg-autonomi-blue-600'
                  }`"
                ></div>
              </div>
              <div
                class="col-span-12 flex gap-10 text-sm text-autonomi-text-primary lg:hidden"
              >
                <div>{{ (Math.random() * 20).toFixed(2) }} GB</div>
                <div>{{ (Math.random() * 12).toFixed(0) }} of 265</div>
                <div>{{ (Math.random() * 100).toFixed(2) }}% complete</div>
              </div>
            </div>

            <!-- Row Menu -->
            <div class="hidden lg:block self-center">
              <i
                class="pi pi-ellipsis-h cursor-pointer hover:text-autonomi-gray-600"
                @click="
                  ($event) => {
                    selectedDownloadItem = item;
                    handleToggleDownloadMenu($event);
                  }
                "
              />
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- MENUS -->
    <!-- FILES VIEW MENU POPOVER -->
    <Popover ref="refFilesViewMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
              v-for="item in menuFilesView"
              :key="item.label"
              class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
              @click="item.command"
            >
              <i :class="item.icon" />
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- FILES MENU POPOVER -->
    <Popover ref="refFilesMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
              v-for="item in menuFiles"
              :key="item.label"
              class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
              @click="item.command"
            >
              <i :class="item.icon" />
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- UPLOAD MENU POPOVER -->
    <Popover ref="refUploadMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
              v-for="item in menuUploads"
              :key="item.label"
              class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
              @click="item.command"
            >
              <i :class="item.icon" />
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- DOWNLOAD MENU POPOVER -->
    <Popover ref="refDownloadMenu" class="syslog-menu">
      <div class="flex flex-col gap-4">
        <div>
          <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
            <li
              v-for="item in menuDownloads"
              :key="item.label"
              class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
              @click="item.command"
            >
              <i :class="item.icon" />
              <div>
                {{ item.label }}
              </div>
            </li>
          </ul>
        </div>
      </div>
    </Popover>

    <!-- DRAWER -->
    <Drawer
      v-model:visible="isVisibleFileInfo"
      header="Drawer"
      position="right"
    >
      <template #header>
        <div class="flex items-center gap-3">
          <div
            class="w-10 h-10 bg-autonomi-gray-500 rounded-full flex items-center justify-center"
          >
            <i class="pi pi-file text-white" />
          </div>
          <div class="text-lg font-semibold text-autonomi-blue-600">
            Details
          </div>
        </div>
      </template>
      <div class="p-5 flex-col flex text-sm font-semibold">
        <div class="py-3">
          <div>Name</div>
          <div class="text-autonomi-text-primary">
            {{ selectedFileItem.name }}
          </div>
        </div>

        <!--        <div class="py-3">-->
        <!--          <div>Type</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.type }}-->
        <!--          </div>-->
        <!--        </div>-->

        <!--        <div class="py-3">-->
        <!--          <div>Size</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.size }}-->
        <!--          </div>-->
        <!--        </div>-->

        <!--        <div class="py-3">-->
        <!--          <div>Storage used</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.storage }}-->
        <!--          </div>-->
        <!--        </div>-->

        <div class="py-3">
          <div>Modified</div>
          <div class="text-autonomi-text-primary">
            {{
              selectedFileItem.dateModified
                ? secondsToDate(selectedFileItem.dateModified).toLocaleString()
                : ""
            }}
          </div>
        </div>

        <!--        <div class="py-3">-->
        <!--          <div>Opened</div>-->
        <!--          <div class="text-autonomi-text-primary">-->
        <!--            {{ selectedFileItem.opened }}-->
        <!--          </div>-->
        <!--        </div>-->

        <div class="py-3">
          <div>Created</div>
          <div class="text-autonomi-text-primary">
            {{
              selectedFileItem.dateCreated
                ? secondsToDate(selectedFileItem.dateCreated).toLocaleString()
                : ""
            }}
          </div>
        </div>
      </div>
    </Drawer>
  </div>
</template>
