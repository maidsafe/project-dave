<script lang="ts" setup>
import {useToast} from "primevue/usetoast";
import {invoke} from "@tauri-apps/api/core";
import {open} from "@tauri-apps/plugin-shell";
import {onMounted} from "vue";
import {toRaw} from "vue";

type SettingsView =
    | "network-settings"
    | "receiver-settings"
    | "syslog"
    | "other-settings";
const refSyslogMenu = ref();
const selectedSyslog = ref<any>(null); // TODO: Replace with actual type
const view = ref<SettingsView>("network-settings");
const toast = useToast();

const isEditForwarders = ref(false);
const isEditEventCollector = ref(false);
const navButtons = ref([
  {
    name: "Network Settings",
    view: "network-settings",
    command: () => {
      view.value = "network-settings";
    },
  },
  // {
  //   name: "Receiver Settings",
  //   view: "receiver-settings",
  //   command: () => {
  //     view.value = "receiver-settings";
  //   },
  // },
  // {
  //   name: "Syslog",
  //   view: "syslog",
  //   command: () => {
  //     view.value = "syslog";
  //   },
  // },
  // {
  //   name: "Other settings",
  //   view: "other-settings",
  //   command: () => {
  //     view.value = "other-settings";
  //   },
  // },
]);

const supportLinks = ref([
  {
    name: "Autonomi.com",
    link: "http://autonomi.com",
    icon: "pi pi-link",
    target: "_blank",
  },
  {
    name: "Discord",
    link: "https://discord.com/invite/autonomi",
    target: "_blank",
    icon: "pi pi-discord",
  },
  {
    name: "Forum",
    link: "https://forum.autonomi.community/",
    icon: "pi pi-link",
    target: "_blank",
  },
  {
    name: "Follow on X",
    link: "https://x.com/WithAutonomi",
    target: "_blank",
    icon: "pi pi-twitter",
  },
]);

const dataSyslog = ref([
  {
    id: 1,
    port: 10001,
    sourceType: "cisco",
    rcf: "RFC 3424",
    transport: "TCP, UDP",
    defaultSource: "xxxx",
  },
  {
    id: 2,
    port: 10001,
    sourceType: "pan",
    rcf: "RFC 1254",
    transport: "UDP",
    defaultSource: "xxxx",
  },
  {
    id: 3,
    port: 10001,
    sourceType: "cisco",
    rcf: "RFC 3424",
    transport: "TPC, UDP",
    defaultSource: "xxxx",
  },
]);

const handleEditForwarders = () => {
  isEditForwarders.value = true;

  toast.add({
    severity: "info",
    summary: "TODO: Edit forwarders",
    detail: "Editing forwarders",
    life: 3000,
  });
};

const handleEditEventCollector = () => {
  isEditEventCollector.value = true;

  toast.add({
    severity: "info",
    summary: "TODO: Edit event collector",
    detail: "Editing event collector",
    life: 3000,
  });
};

const handleToggleSyslogMenu = (event: MouseEvent) => {
  refSyslogMenu.value.toggle(event);
};

const handleEditSyslogItem = () => {
  toast.add({
    severity: "info",
    summary: "Edit syslog item",
    detail: `Editing syslog item: ${selectedSyslog.value?.id}`,
    life: 3000,
  });
};

const handleDeleteSyslogItem = () => {
  toast.add({
    severity: "warn",
    summary: "Delete syslog item",
    detail: `Deleting syslog item: ${selectedSyslog.value?.id}`,
    life: 3000,
  });
};

const menuSyslog = ref([
  {
    label: "Edit",
    icon: "pi pi-pencil",
    command: handleEditSyslogItem,
  },
  {
    label: "Delete",
    icon: "pi pi-trash",
    command: handleDeleteSyslogItem,
  },
]);

const handleUpdatePortNumber = () => {
  toast.add({
    severity: "success",
    summary: "TODO: Port number changed",
    detail: "Port number has been successfully changed",
    life: 3000,
  });

  // isEditForwarders.value = false;
};

const handleUpdateMaxChannels = () => {
  toast.add({
    severity: "success",
    summary: "TODO: Max Channels changed",
    detail: "Max channels has been successfully changed",
    life: 3000,
  });

  // isEditForwarders.value = false;
};

const handleUpdateEventCollector = () => {
  toast.add({
    severity: "success",
    summary: "TODO: Event Collector changed",
    detail: "Event collector has been successfully changed",
    life: 3000,
  });

  // isEditEventCollector.value = false;
};

async function saveSettingsButtonHandler() {
  try {
    await invoke("app_data_store", {
      appData: {
        download_path: downloadPath.value,
        peers: bootstrapPeers.value.split(","),
      },
    });
  } catch (e) {
    console.error(e);
    saveSettingsErrorMessage.value = e as any; // TODO: DOES NOT UPDATE?!?! :(
  } finally {
    saveSettingsErrorMessage.value = "";
  }
}

const downloadPath = ref("");
const bootstrapPeers = ref("");
const saveSettingsErrorMessage = ref("");
const name = ref("");

async function loadSettings() {
  let app_data: any = await invoke("app_data");
  downloadPath.value = app_data.download_path;
  bootstrapPeers.value = app_data.peers.join(",");
}

onMounted(async () => {
  try {
    const response = await loadSettings();
    console.log(">>> Settings response: ", response);
  } catch (e) {
    console.log(">>> Error loading settings: ", e);
  }
});
</script>

<template>
  <div class="pr-[66px] pl-[110px] mt-10">
    <!-- <div class="text-2xl font-semibold flex gap-20">
      <template v-for="button in navButtons" :key="button.name">
        <button
          @click="button.command"
          :class="`${
            view === button.view ? 'text-autonomi-blue-600' : 'text-gray-500'
          } transition-all duration-300`"
        >
          {{ button.name }}
        </button>
      </template>
    </div> -->

    <!-- Views -->
    <div class="-mr-[66px] -ml-[110px] mt-10">
      <!-- View: Other settings -->
      <!-- <div v-if="view === 'network-settings'">
        <div class="pr-[66px] pl-[110px] py-7">
          <div class="flex justify-between items-center gap-10">
            <div>
              <h3 class="text-2xl text-autonomi-header-text-dark font-semibold">
                Bootstrap Peers
              </h3>
              separated by comma <hr>
              <Textarea v-model="bootstrapPeers" />
              <h3 class="text-2xl text-autonomi-header-text-dark font-semibold">
                Download folder
              </h3>
              <InputText
              v-model="downloadPath"
                class="font-semibold bg-autonomi-gray-500 border-none text-autonomi-text-primary text-center"
                placeholder="Download folder"
              />
              <button @click="saveSettingsButtonHandler">Save settings</button>

              <div>{{ saveSettingsErrorMessage }}</div>
            </div>
          </div>
        </div>
      </div> -->

      <!-- View: Receiver Settings -->
      <div v-if="view === 'receiver-settings'">
        <!-- Autonomi forwarders -->
        <div
            class="flex items-center justify-between pr-[66px] pl-[110px] py-7 bg-autonomi-gray-100 gap-10"
        >
          <div>
            <h3 class="text-autonomi-header-text-dark text-lg font-semibold">
              Autonomi forwarders
            </h3>
            <p class="text-autonomi-text-primary">
              Config settings for edge processors to receive data from universal
              or heavy forwarders.
            </p>
          </div>
          <div>
            <CommonButton @click="handleEditForwarders" variant="tertiary">
              Edit
            </CommonButton>
          </div>
        </div>

        <!-- Port & Channels -->
        <div
            class="flex flex-col items-center justify-between pr-[66px] pl-[110px] py-7 text-sm"
        >
          <div class="w-full flex flex-col gap-y-4">
            <div class="flex flex-wrap gap-y-4">
              <div
                  class="text-autonomi-header-text-dark text-sm font-semibold w-[200px]"
              >
                Port
              </div>
              <p class="text-autonomi-text-primary col-span-10">8799</p>
            </div>

            <div class="flex flex-wrap gap-y-4">
              <div
                  class="text-autonomi-header-text-dark text-sm font-semibold w-[200px]"
              >
                Maximum channels
              </div>
              <p class="text-autonomi-text-primary col-span-10">300</p>
            </div>
          </div>
        </div>

        <!-- HTTP Event collector forwarders -->
        <div
            class="flex items-center justify-between pr-[66px] pl-[110px] py-7 bg-autonomi-gray-100 gap-10"
        >
          <div>
            <h3 class="text-autonomi-header-text-dark text-lg font-semibold">
              HTTP Event Collector
            </h3>
            <p class="text-autonomi-text-primary">
              Config settings for the edge processor to receive data from
              logging agents & HTTP clients via HTTP Event Collector.
            </p>
          </div>
          <div>
            <CommonButton @click="handleEditEventCollector" variant="tertiary">
              Edit
            </CommonButton>
          </div>
        </div>

        <!-- FORWARDERS DRAWER -->
        <Drawer
            v-model:visible="isEditForwarders"
            header="Forwarders"
            position="right"
            class="!h-auto !w-[380px] rounded-l-2xl"
        >
          <div
              class="border-t border-t-autonomi-text-primary/10 flex flex-col items-center py-7 w-[80%] mx-auto"
          >
            <h3 class="text-lg text-autonomi-header-text-dark font-semibold">
              Port Number
            </h3>
            <p
                class="text-autonomi-text-primary mt-2 text-center max-w-[70%] text-xs"
            >
              Choose the start of the range below, edit this text to suit.
            </p>

            <div class="flex items-center gap-2 font-semibold mt-4">
              <InputText
                  value="3001"
                  class="w-[70px] font-semibold bg-autonomi-gray-500 border-none text-autonomi-text-primary text-center"
                  placeholder="Port number"
              />
              <span class="text-autonomi-text-secondary font-semibold"
              >to 3999</span
              >
            </div>

            <CommonButton
                variant="secondary"
                class="mt-4"
                @click="handleUpdatePortNumber"
            >
              Confirm
            </CommonButton>
          </div>

          <div
              class="border-t border-t-autonomi-text-primary/10 flex flex-col items-center py-7 w-[80%] mx-auto"
          >
            <h3 class="text-lg text-autonomi-header-text-dark font-semibold">
              Max Channels
            </h3>
            <p
                class="text-autonomi-text-primary mt-2 text-center max-w-[70%] text-xs"
            >
              Choose the start of the range below, edit this text to suit.
            </p>

            <div class="flex items-center gap-2 font-semibold mt-4">
              <span class="text-autonomi-text-secondary font-semibold"
              >100 to</span
              >
              <InputText
                  value="300"
                  class="w-[70px] font-semibold bg-autonomi-gray-500 border-none text-autonomi-text-primary text-center"
                  placeholder="Port number"
              />
            </div>

            <CommonButton
                variant="secondary"
                class="mt-4"
                @click="handleUpdateMaxChannels"
            >
              Confirm
            </CommonButton>
          </div>
        </Drawer>

        <!-- EVENT COLLECTOR DRAWER -->
        <Drawer
            v-model:visible="isEditEventCollector"
            header="Event Collector"
            position="right"
            class="!h-auto !w-[380px] rounded-l-2xl"
        >
          <div
              class="border-t border-t-autonomi-text-primary/10 flex flex-col items-center py-7 w-[80%] mx-auto"
          >
            <h3 class="text-lg text-autonomi-header-text-dark font-semibold">
              Event Collector
            </h3>
            <p
                class="text-autonomi-text-primary mt-2 text-center max-w-[70%] text-xs"
            >
              Choose the start of the range below, edit this text to suit.
            </p>

            <div class="flex items-center gap-2 font-semibold mt-4">
              <InputText
                  value="8799"
                  class="w-[70px] font-semibold bg-autonomi-gray-500 border-none text-autonomi-text-primary text-center"
                  placeholder="Port number"
              />
              <span class="text-autonomi-text-secondary font-semibold"
              >to 9099</span
              >
            </div>

            <CommonButton
                variant="secondary"
                class="mt-4"
                @click="handleUpdateEventCollector"
            >
              Confirm
            </CommonButton>
          </div>
        </Drawer>
      </div>

      <!-- View: Syslog -->
      <div v-if="view === 'syslog'">
        <div class="pr-[66px] pl-[110px] py-7">
          <div class="flex justify-between items-center gap-10">
            <div>
              <h3 class="text-2xl text-autonomi-header-text-dark font-semibold">
                Syslog
              </h3>
              <p class="text-autonomi-text-primary mt-2">
                Config settings for the edge processor to receive data from
                syslog agents.
              </p>
            </div>

            <div>
              <CommonButton
                  @click="handleEditForwarders"
                  variant="secondary"
                  size="medium"
              >
                <i class="pi pi-plus"/> Add port
              </CommonButton>
            </div>
          </div>
        </div>

        <!-- Syslog Table -->
        <div>
          <!-- Header -->
          <div class="pr-[66px] pl-[110px] mb-4">
            <div class="grid grid-cols-12">
              <div class="col-span-2 text-autonomi-red-300 font-semibold">
                Port
              </div>
              <div class="col-span-2 text-autonomi-red-300 font-semibold">
                Source type
              </div>
              <div class="col-span-2 text-autonomi-red-300 font-semibold">
                RCF
              </div>
              <div class="col-span-2 text-autonomi-red-300 font-semibold">
                Transport
              </div>
              <div class="col-span-2 text-autonomi-red-300 font-semibold">
                Default Source
              </div>
            </div>
          </div>

          <!-- Body -->
          <template v-for="log in dataSyslog" :key="log.id">
            <div
                class="pr-[66px] pl-[110px] py-4 grid grid-cols-12 even:bg-autonomi-gray-100"
            >
              <div class="col-span-2 text-autonomi-text-primary">
                {{ log.port }}
              </div>
              <div class="col-span-2 text-autonomi-text-primary">
                {{ log.sourceType }}
              </div>
              <div class="col-span-2 text-autonomi-text-primary">
                {{ log.rcf }}
              </div>
              <div class="col-span-2 text-autonomi-text-primary">
                {{ log.transport }}
              </div>
              <div class="col-span-2 text-autonomi-text-primary">
                {{ log.defaultSource }}
              </div>
              <div class="col-span-2 flex justify-end">
                <button
                    @click="
                    ($event) => {
                      selectedSyslog = log;
                      handleToggleSyslogMenu($event);
                    }
                  "
                >
                  <i class="pi pi-ellipsis-v"/>
                </button>
              </div>
            </div>
          </template>
        </div>

        <!-- Menu Popover -->
        <Popover ref="refSyslogMenu" class="syslog-menu">
          <div class="flex flex-col gap-4">
            <div>
              <ul class="list-none p-0 m-0 flex flex-col min-w-[150px]">
                <li
                    v-for="item in menuSyslog"
                    :key="item.label"
                    class="flex items-center gap-2 py-3 px-5 hover:bg-autonomi-gray-100 cursor-pointer rounded-border rounded-2xl"
                    @click="item.command"
                >
                  <i :class="item.icon"/>
                  <div>
                    {{ item.label }}
                  </div>
                </li>
              </ul>
            </div>
          </div>
        </Popover>
      </div>

      <!-- View: Other settings -->
      <div v-if="view === 'other-settings'">
        <div class="pr-[66px] pl-[110px] py-7">
          <div class="flex justify-between items-center gap-10">
            <div>
              <h3 class="text-2xl text-autonomi-header-text-dark font-semibold">
                Other Settings
              </h3>
              <p class="text-autonomi-text-primary mt-2">
                Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do
                eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut
                enim ad minim veniam, quis nostrud exercitation ullamco laboris
                nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor
                in reprehenderit in voluptate velit esse cillum dolore eu fugiat
                nulla pariatur. Excepteur sint occaecat cupidatat non proident,
                sunt in culpa qui officia deserunt mollit anim id est laborum.
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Help & Support -->
      <div class="pr-[66px] pl-[110px] mt-12">
        <h3 class="text-2xl text-autonomi-header-text-dark dark:text-autonomi-text-primary-dark font-semibold">
          About us
        </h3>

        <div class="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 mt-7">
          <button
              v-for="link in supportLinks"
              @click="() => open(link.link)"
              class="text-autonomi-text-primary font-semibold flex flex-col items-center justify-center border-autonomi-text-primary/50 border rounded-lg h-24 bg-white/30 dark:bg-white/90 text-center gap-2 hover:bg-white transition-all duration-300 hover:border-autonomi-text-primary px-3 cursor-pointer"
          >
            <i :class="`${link.icon} text-autonomi-blue-600 mr-1`"/>
            {{ link.name }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="css">
.syslog-menu.p-popover.p-component:before,
.syslog-menu.p-popover.p-component:after {
  display: none;
}
</style>
