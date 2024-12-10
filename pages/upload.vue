<script lang="ts" setup>
import { usePrimeVue } from "primevue/config";
import { useToast } from "primevue/usetoast";
import { getCurrentDateTimeString } from "public/scripts/utils";

const $primevue = usePrimeVue();
const toast = useToast();
// const autonomi = useAutonomiStore();

const totalSize = ref(0);
const totalSizePercent = ref(0);
const files = ref([]);
const isGeneratingQuote = ref(false);
const isVisibleQuoteGen = ref(false);
const isVisiblePayAndUpload = ref(false);
const fileupload = ref();

const quotesResult = ref();

const onRemoveTemplatingFile = (file, removeFileCallback, index) => {
  removeFileCallback(index);
  totalSize.value -= parseInt(formatSize(file.size));
  totalSizePercent.value = totalSize.value / 10;
};

const onClearTemplatingUpload = () => {
  console.log(">>> Running clear");
  files.value = [];
  fileupload.value.clear();
  totalSize.value = 0;
  totalSizePercent.value = 0;
};

const handleClearUploads = (clearCallback) => {
  clearCallback();
  totalSize.value = 0;
  totalSizePercent.value = 0;
  files.value = [];
  console.log(">>> clearHandled");
};

const onSelectedFiles = (event) => {
  try {
    console.log(">>> files selected", event);
    files.value = event.files;
    files.value.forEach((file) => {
      totalSize.value += parseInt(formatSize(file.size));
    });
  } catch (error) {
    console.log(">>> Error selecting file: ", error);
  }
};

const uploadEvent = (callback) => {
  totalSizePercent.value = totalSize.value / 10;
  callback();
};

const onTemplatedUpload = () => {
  toast.add({
    severity: "info",
    summary: "Success",
    detail: "File Uploaded",
    life: 3000,
  });
};

const formatSize = (bytes) => {
  const k = 1024;
  const dm = 3;
  const sizes = $primevue.config.locale.fileSizeTypes;

  if (bytes === 0) {
    return `0 ${sizes[0]}`;
  }

  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const formattedSize = parseFloat((bytes / Math.pow(k, i)).toFixed(dm));

  return `${formattedSize} ${sizes[i]}`;
};

const readFileToBytes = (file) => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    // Read the file as an ArrayBuffer
    reader.readAsArrayBuffer(file);

    reader.onload = function (event) {
      const arrayBuffer = event.target.result; // Get the ArrayBuffer
      const uint8Array = new Uint8Array(arrayBuffer); // Convert to Uint8Array
      resolve(uint8Array); // Resolve the promise with Uint8Array
    };

    reader.onerror = function (event) {
      reject(
        new Error("File could not be read! Code " + event.target.error.code)
      );
    };
  });
};

const fetchQuotesForFiles = async (): Promise<{
  quotes: Map<any, any>;
  quotePayments: any[];
  freeChunks: any[];
}> => {
  let combinedQuotes = new Map();
  let combinedQuotePayments = [];
  let combinedFreeChunks = [];

  for (const file of files.value) {
    const fileBytes = await readFileToBytes(file);

    // Encrypt the data to chunks
    const {
      dataMapChunk,
      dataChunks,
      dataMapChunkAddress,
      dataChunkAddresses,
    } = autonomi.encryptData(fileBytes);

    // Get quotes
    const { quotes, quotePayments, freeChunks } =
      await autonomi.getQuotesForContentAddrs(dataChunkAddresses);

    quotes.forEach((v, k) => combinedQuotes.set(k, v));
    combinedQuotePayments = combinedQuotePayments.concat(quotePayments);
    combinedFreeChunks = combinedFreeChunks.concat(freeChunks);
  }

  return {
    quotes: combinedQuotes,
    quotePayments: combinedQuotePayments,
    freeChunks: combinedFreeChunks,
  };
};

const handleGenerateQuote = async () => {
  isGeneratingQuote.value = true;

  try {
    const { quotes, quotePayments, freeChunks } = await fetchQuotesForFiles();

    quotesResult.value = { quotes, quotePayments, freeChunks };

    isVisibleQuoteGen.value = true;

    toast.add({
      severity: "info",
      summary: "Success",
      detail: "Quote Generated",
      life: 3000,
    });
  } catch (error) {
    console.log(">>> Error while getting quotes: ", error);
  } finally {
    isGeneratingQuote.value = false;
  }
};

const handlePayAndUpload = async () => {
  isVisibleQuoteGen.value = false;
  isVisiblePayAndUpload.value = true;

  try {
    // Use the current datetime as archive name
    let archiveName = getCurrentDateTimeString();

    let filesToUpload = [];

    for (const file of files.value) {
      const fileName = file.name;
      const fileBytes = await readFileToBytes(file);

      filesToUpload.push({
        name: fileName,
        bytes: fileBytes,
      });
    }

    // // Try to reuse quotes if available
    // if (quotesResult.value) {
    //   let receipt = autonomi.executeQuotePayments(quotesResult.value.quotes, quotesResult.value.quotePayments);
    //   await autonomi.putFilesToVault(filesToUpload, archiveName, receipt);
    // } else {
    //   await autonomi.putFilesToVault(filesToUpload, archiveName);
    // }

    await autonomi.putFilesToVault(filesToUpload, archiveName);

    onClearTemplatingUpload();

    toast.add({
      severity: "info",
      summary: "Success",
      detail: "File(s) Uploaded",
      life: 3000,
    });

    await navigateTo("/");
  } catch (error) {
    console.log(">>> Error while uploading data: ", error);

    toast.add({
      severity: "error",
      summary: "Error",
      detail: error,
      life: 3000,
    });
  }

  isVisiblePayAndUpload.value = false;
};

const handleCancelGenerateQuote = () => {
  isVisibleQuoteGen.value = false;
  isGeneratingQuote.value = false;
};

const handleUploadError = (error) => {
  console.log(">>> Error", error);
};

const upload = () => {
  console.log(">>> Running upload:");
  fileupload.value.upload();
};

const handleChooseUpload = () => {
  console.log(">>> Running choose upload");
  fileupload.value.choose();
};

const quoteBreakdownData = computed(() => {
  if (!quotesResult.value) return null;

  let freeChunks = quotesResult.value.freeChunks;
  let chunksAmount = quotesResult.value.quotes.size;
  let totalPrice = quotesResult.value.quotePayments.reduce(
    (acc, v) => acc + Number(v[2]),
    0
  );
  let avgChunkPrice = totalPrice / chunksAmount;

  return { freeChunks, chunksAmount, totalPrice, avgChunkPrice };
});
</script>

<template>
  <div>
    <!-- UPLOADER PAGE -->
    <div
      class="autonomi-uploader px-[100px] py-[70px]"
      :class="`${
        !isGeneratingQuote ? '' : 'hidden'
      } transition-all duration-300`"
    >
      <!-- <Toast /> -->

      <div
        :class="`${
          files.length ? 'h-[80px]' : 'h-0'
        } overflow-hidden transition-all duration-300 flex items-start`"
      >
        <CommonButton variant="secondary" @click="handleGenerateQuote">
          Generate Quote
        </CommonButton>
      </div>

      <FileUpload
        name="demo[]"
        url="/api/upload"
        @upload="onAdvancedUpload($event)"
        :multiple="true"
        @select="onSelectedFiles"
        @error="handleUploadError"
        ref="fileupload"
      >
        <template
          #header="{ chooseCallback, uploadCallback, clearCallback, files }"
        >
          <div class="flex flex-wrap justify-between items-center flex-1 gap-4">
            <div class="flex gap-2">
              <Button
                @click="chooseCallback()"
                icon="pi pi-images"
                rounded
                outlined
                severity="secondary"
              ></Button>
              <!-- <Button
                @click="uploadEvent(uploadCallback)"
                icon="pi pi-cloud-upload"
                rounded
                outlined
                severity="success"
                :disabled="!files || files.length === 0"
              ></Button> -->
              <Button
                @click="handleClearUploads(clearCallback)"
                icon="pi pi-times"
                rounded
                outlined
                severity="danger"
                :disabled="!files || files.length === 0"
              ></Button>
            </div>
            <!-- <ProgressBar
              :value="totalSizePercent"
              :showValue="false"
              class="md:w-20rem h-1 w-full md:ml-auto"
            >
              <span class="whitespace-nowrap">{{ totalSize }}B / 1Mb</span>
            </ProgressBar> -->
          </div>
        </template>
        <template
          #content="{
            files,
            messages,
            uploadedFiles,
            removeUploadedFileCallback,
            removeFileCallback,
            ...rest
          }"
        >
          <div class="flex flex-col gap-8 pt-4">
            <div v-if="messages?.length">
              <div
                v-for="message in messages"
                class="text-xs bg-red-400 text-white py-2 px-6 rounded-lg border border-red-600"
              >
                {{ message }}
              </div>
            </div>
            <div v-if="files.length > 0">
              <!-- <h5
                class="bg-green-500 py-2 px-6 text-xs rounded-lg text-white mb-6"
              >
                <span class="animate-pulse">Pending</span>
              </h5> -->
              <div class="flex flex-wrap gap-4">
                <div
                  v-for="(file, index) of files"
                  :key="file.name + file.type + file.size"
                  class="p-8 rounded-border flex flex-col border border-surface items-center gap-4"
                >
                  <div>
                    <i
                      v-if="file?.type === 'application/pdf'"
                      class="pi pi-file-pdf text-5xl"
                    />
                    <img
                      v-else
                      role="presentation"
                      :alt="file.name"
                      :src="file.objectURL"
                      width="100"
                      height="50"
                    />
                  </div>
                  <span
                    class="font-semibold text-ellipsis max-w-60 whitespace-nowrap overflow-hidden mt-auto"
                    >{{ file.name }}
                  </span>
                  <div>{{ formatSize(file.size) }}</div>
                  <Badge value="Pending" severity="warn" />
                  <Button
                    icon="pi pi-times"
                    @click="
                      onRemoveTemplatingFile(file, removeFileCallback, index)
                    "
                    outlined
                    rounded
                    severity="danger"
                  />
                </div>
              </div>
            </div>

            <div v-if="uploadedFiles.length > 0">
              <h5>Completed</h5>
              <div class="flex flex-wrap gap-4">
                <div
                  v-for="(file, index) of uploadedFiles"
                  :key="file.name + file.type + file.size"
                  class="p-8 rounded-border flex flex-col border border-surface items-center gap-4"
                >
                  <div>
                    <img
                      role="presentation"
                      :alt="file.name"
                      :src="file.objectURL"
                      width="100"
                      height="50"
                    />
                  </div>
                  <span
                    class="font-semibold text-ellipsis max-w-60 whitespace-nowrap overflow-hidden"
                    >{{ file.name }}</span
                  >
                  <div>{{ formatSize(file.size) }}</div>
                  <Badge value="Completed" class="mt-4" severity="success" />
                  <Button
                    icon="pi pi-times"
                    @click="removeUploadedFileCallback(index)"
                    outlined
                    rounded
                    severity="danger"
                  />
                </div>
              </div>
            </div>

            <!-- TEST DATA -->
            <!-- <code class="text-xs bg-slate-200 rounded-lg p-6 flex flex-col gap-3">
              <div class="font-semibold">Test Data:</div>
              <div>{{ messages }}</div>
              <div>{{ rest }}</div>
            </code> -->
          </div>
        </template>
        <template #empty="{ chooseCallback }">
          <div class="flex items-center justify-center flex-col h-full">
            <i
              class="pi pi-cloud-upload !border-0 !rounded-full !p-8 !text-[100px] !text-autonomi-gray-500"
            />
            <p
              class="mt-2 mb-0 text-4xl font-semibold text-autonomi-text-primary"
            >
              <span class="lg:hidden">upload files</span>
              <span class="hidden lg:block">Drag & Drop files here</span>
            </p>

            <div class="mt-6 lg:hidden">
              <button
                @click="handleChooseUpload"
                class="bg-autonomi-red-300 text-white px-8 py-2 rounded-full"
              >
                <i class="pi pi-plus-circle" /> Upload
              </button>
            </div>
          </div>
        </template>
      </FileUpload>

      <div
        :class="`${
          files.length ? 'h-[80px]' : 'h-0'
        } overflow-hidden transition-all duration-300 flex items-start mt-10`"
      >
        <CommonButton variant="secondary" @click="handleGenerateQuote">
          Generate Quote
        </CommonButton>
      </div>
    </div>

    <!-- GENERATING NOTICE -->
    <div
      class="px-[100px] py-[70px] items-center justify-center gap-10 min-h-[560px]"
      :class="`${
        isGeneratingQuote ? 'flex flex-col' : 'hidden'
      } transition-all duration-300`"
    >
      <i class="pi pi-spin pi-spinner text-6xl text-autonomi-gray-200" />
      <div class="text-autonomi-text-primary text-4xl font-semibold">
        Gathering your quote
      </div>
    </div>

    <!-- QUOTE DRAWER -->
    <Drawer
      v-model:visible="isVisibleQuoteGen"
      header="Quote Breakdown"
      position="right"
      class="!h-auto !w-[380px] rounded-l-2xl"
    >
      <!--      <p>-->
      <!--        Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod-->
      <!--        tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim-->
      <!--        veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea-->
      <!--        commodo consequat.-->
      <!--      </p>-->

      <div class="text-xs font-semibold">
        <div>
          <div class="text-aut text-autonomi-text-secondary">Chunks</div>
          <div class="text-autonomi-text-primary">
            {{ quoteBreakdownData?.chunksAmount }}
          </div>
        </div>
        <div class="border-t border-autonomi-text-primary/30 my-4" />
        <div>
          <div class="text-aut text-autonomi-text-secondary">
            Avg. Price per chunk
          </div>
          <div class="text-autonomi-text-primary">
            {{ quoteBreakdownData?.avgChunkPrice }} Atto
          </div>
        </div>
        <div class="border-t border-autonomi-text-primary/30 my-4" />
        <div>
          <div class="text-aut text-autonomi-text-secondary">
            Total Price (Ex. gas fee)
          </div>
          <div class="text-autonomi-text-primary">
            {{ quoteBreakdownData?.totalPrice }} Atto
          </div>
        </div>
        <div class="border-t border-autonomi-text-primary/30 my-4" />
      </div>

      <div class="flex items-center justify-center mt-8 gap-4">
        <CommonButton variant="secondary" @click="handlePayAndUpload">
          Pay &amp; Upload
        </CommonButton>
        <CommonButton variant="tertiary" @click="handleCancelGenerateQuote">
          Cancel
        </CommonButton>
      </div>
    </Drawer>

    <!-- PAY & UPLOAD DRAWER -->
    <Drawer
      v-model:visible="isVisiblePayAndUpload"
      header="Uploading File(s)"
      position="right"
      class="!h-auto !w-[380px] rounded-l-2xl"
    >
      <template v-if="autonomi.uploadStep === 'network'">
        <p class="text-xs font-semibold">Connecting to network..</p>
      </template>
      <template v-else-if="autonomi.uploadStep === 'file'">
        <p class="text-xs font-semibold">
          Uploading file {{ 1 + autonomi.filesUploaded }} of
          {{ files.length }} ({{ autonomi.fileBeingUploaded }})..
        </p>

        <div class="mt-2">
          <template v-if="autonomi.state === 'encrypting'">
            <div>Encrypting file..</div>
          </template>
          <template v-else-if="autonomi.state === 'quoting'">
            <div>Fetching quotes for file..</div>
          </template>
          <template v-else-if="autonomi.state === 'approving'">
            <div>Approving token spend allowance..</div>
          </template>
          <template v-else-if="autonomi.state === 'paying'">
            <div>Approving chunk payments..</div>
          </template>
          <template v-else-if="autonomi.state === 'uploading'">
            <div>Storing file chunks..</div>
          </template>
        </div>
      </template>
      <template v-else-if="autonomi.uploadStep === 'archive'">
        <p class="text-xs font-semibold">Uploading archive..</p>

        <div class="mt-2">
          <template v-if="autonomi.state === 'encrypting'">
            <div>Encrypting archive..</div>
          </template>
          <template v-else-if="autonomi.state === 'quoting'">
            <div>Fetching quotes for archive..</div>
          </template>
          <template v-else-if="autonomi.state === 'approving'">
            <div>Approving token spend allowance..</div>
          </template>
          <template v-else-if="autonomi.state === 'paying'">
            <div>Approving chunk payments..</div>
          </template>
          <template v-else-if="autonomi.state === 'uploading'">
            <div>Storing archive chunks..</div>
          </template>
        </div>
      </template>
      <template v-else-if="autonomi.uploadStep === 'vault'">
        <p class="text-xs font-semibold">Updating vault..</p>

        <div class="mt-2">
          <template v-if="autonomi.state === 'vaultFetching'">
            <div>Fetching vault..</div>
          </template>
          <template v-else>
            <div>Uploading updated vault..</div>
          </template>
        </div>
      </template>
    </Drawer>

    <!-- FILE VIEWER -->
    <!-- <FileViewer /> -->
  </div>
</template>

<style lang="scss">
.autonomi-uploader {
  .p-fileupload.p-fileupload-advanced {
    @apply bg-none bg-transparent border-4 border-autonomi-gray-200 border-none lg:border-dashed min-h-[560px];

    .p-fileupload-content {
      @apply min-h-[440px];
    }
  }
}
</style>
