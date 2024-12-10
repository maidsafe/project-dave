/*
****** TEMP COMMENT ******
**************************

import init, * as autonomi from "public/scripts/autonomi/autonomi";
import {sendTransaction, signMessage, waitForTransactionReceipt} from "@wagmi/core";
import {config} from "~/config";

const VAULT_SECRET_KEY_SEED = "Massive Array of Internet Disks Secure Access For Everyone";
const TX_CONFIRMATION_TIME_MS = 5000;

// TODO: set this using ENV var
const DEFAULT_SAFE_PEER = "/ip4/167.71.140.72/tcp/37462/ws/p2p/12D3KooWMrQ95QFLzKkyTc2X3aEFqGhH5cPmsf1QkCuAD4PWNqyW";

export const useAutonomiStore = defineStore("autonomi", () => {
  const initializedWasm = ref<boolean>(false);
  const savedClient = ref<autonomi.Client>();
  const savedVaultKey = ref<string>();

  const state = ref<"encrypting" | "quoting" | "approving" | "paying" | "uploading" | "vaultFetching" | "vaultUpdating">();

  const fileBeingUploaded = ref<string>();
  const filesUploaded = ref(0);
  const uploadStep = ref<"network" | "file" | "archive" | "vault">("files");

  const initWasm = async () => {
    if (!initializedWasm.value) {
      await init();
      initializedWasm.value = true;
      // TODO: remove this in prod
      autonomi.logInit("autonomi=trace");
    }
  }

  const connectedClient = async (): Promise<autonomi.Client> => {
    // Return existing Client if available
    if (savedClient.value) return savedClient.value;

    // Connect and return client
    savedClient.value = await autonomi.Client.connect([DEFAULT_SAFE_PEER]);

    return savedClient.value;
  }

  const encryptData = (data: Uint8Array): { dataMapChunk: any, dataChunks: any, dataMapChunkAddress: any, dataChunkAddresses: any } => {
    // Encrypt the data to chunks
    const [dataMapChunk, dataChunks, dataMapChunkAddress, dataChunkAddresses] = autonomi.encryptData(data);

    return { dataMapChunk, dataChunks, dataMapChunkAddress, dataChunkAddresses };
  }

  const getQuotesForContentAddrs = async (contentAddrs: string[]): Promise<{ quotes: any, quotePayments: any, freeChunks: any }> => {
    let client = await connectedClient();

    // Get quotes and payment information
    const [quotes, quotePayments, freeChunks] = await client.getQuotes(contentAddrs);

    return { quotes, quotePayments, freeChunks };
  }

  const putPrivateData = async (data: Uint8Array, receipt?: any): Promise<any> => {
    let client = await connectedClient();

    state.value = "encrypting";

    // Encrypt the data to chunks
    const [dataMapChunk, dataChunks, dataMapChunkAddress, dataChunkAddresses] = autonomi.encryptData(data);

    // Use receipt if it is available
    if (!receipt) {
      state.value = "quoting";

      // Fetch quotes for the chunks
      const [quotes, quotePayments, _freeChunks] = await client.getQuotes(dataChunkAddresses);

      // Skip if already uploaded
      if (quotePayments.length == 0) return dataMapChunk;

      // Pay for data chunks (not the data map)
      receipt = await executeQuotePayments(quotes, quotePayments);

      // Wait for a few seconds to allow tx to confirm
      await new Promise(resolve => setTimeout(resolve, TX_CONFIRMATION_TIME_MS));
    }

    state.value = "uploading";

    // Upload the data, returns PrivateDataAccess chunk
    return await client.putPrivateDataWithReceipt(data, receipt);
  }

  const putPrivateArchive = async (privateArchive: autonomi.PrivateArchive): Promise<any> => {
    let client = await connectedClient();

    state.value = "encrypting";

    // Get the private archive's bytes
    const privateArchiveBytes = privateArchive.bytes();

    // Encrypt the private archive to chunks
    const [paDataMapChunk, paDataChunks, paDataMapChunkAddress, paDataChunkAddresses] = autonomi.encryptData(privateArchiveBytes);

    state.value = "quoting";

    // Fetch quotes for the private archive chunks
    const [paQuotes, paQuotePayments, paFreeChunks] = await client.getQuotes(paDataChunkAddresses);

    // Skip if already uploaded
    if (paQuotePayments.length == 0) return paDataMapChunk;

    state.value = "paying";

    // Pay for the private archive chunks (not the data map)
    const paReceipt = await executeQuotePayments(paQuotes, paQuotePayments);

    // Wait for a few seconds to allow tx to confirm
    await new Promise(resolve => setTimeout(resolve, 5000));

    state.value = "uploading";

    // Uploads the private archive, returns PrivateArchiveAccess chunk
    return await client.putPrivateArchiveWithReceipt(privateArchive, paReceipt);
  }

  const putUserData = async (userData: autonomi.UserData) => {
    let client = await connectedClient();

    state.value = "vaultUpdating";

    let vaultKey = await getVaultKey();

    // Get or create a scratchpad for the user data
    let scratchpad = await client.getOrCreateUserDataScratchpad(vaultKey);

    // Content address of the scratchpad
    let scratchPadAddress = scratchpad.xorName();

    // Fetch quotes for the scratchpad
    const [spQuotes, spQuotePayments, spFreeChunks] = await client.getQuotes(scratchPadAddress ? [scratchPadAddress] : []);

    // Pay for the private archive chunks (not the data map)
    const spReceipt = await executeQuotePayments(spQuotes, spQuotePayments);

    // Wait for a few seconds to allow tx to confirm
    await new Promise(resolve => setTimeout(resolve, 5000));

    // Update vault
    await client.putUserDataToVaultWithReceipt(userData, spReceipt, vaultKey);
  }

  const getUserData = async (): Promise<autonomi.UserData> => {
    let client = await connectedClient();

    let vaultKey = await getVaultKey();

    return await client.getUserDataFromVault(vaultKey);
  }

  const getData = async (dataAddr: string): Promise<Uint8Array> => {
    let client = await connectedClient();

    return await client.getData(dataAddr);
  }

  const getPrivateData = async (privateDataAccess: any): Promise<Uint8Array> => {
    let client = await connectedClient();

    return await client.getPrivateData(privateDataAccess);
  }

  const getVaultKey = async (): Promise<autonomi.SecretKey> => {
    if (!savedVaultKey.value) {
      let signature = await signMessage(config, {
        message: VAULT_SECRET_KEY_SEED
      });

      savedVaultKey.value = autonomi.vaultKeyFromSignature(signature);
    }

    return savedVaultKey.value;
  }

  type PutFilesToVaultInput = [{
    name: string,
    bytes: Uint8Array,
  }]

  const putFilesToVault = async (files: PutFilesToVaultInput, archiveName: string, receipt?: any): Promise<void> => {
    uploadStep.value = "network";

    let client = await connectedClient();

    uploadStep.value = "file";
    filesUploaded.value = 0;

    // Create a private archive
    const privateArchive = new autonomi.PrivateArchive();

    for (const file of files) {
      fileBeingUploaded.value = file.name;

      let fileSize = BigInt(file.bytes?.byteLength ?? 0);

      // Upload the data privately
      let privateDataAccess = await putPrivateData(file.bytes, receipt);

      if (privateDataAccess) {
        // Add our data's data map chunk to the private archive
        privateArchive.addFile(file.name, privateDataAccess, autonomi.createMetadata(fileSize));
      }

      fileBeingUploaded.value = null;
      filesUploaded.value += 1;
    }

    uploadStep.value = "archive";

    // Upload private archive
    let privateArchiveAccess = await putPrivateArchive(privateArchive);

    uploadStep.value = "vault";
    state.value = "vaultFetching";

    // Fetch user data from vault
    let userData;

    try {
      // Throws error if nu user data exists yet
      userData = await getUserData();
    } catch (err) {
      userData = new autonomi.UserData();
    }

    // Add archive to user data
    userData.addPrivateFileArchive(privateArchiveAccess, archiveName);

    // Update the user data vault
    await putUserData(userData);
  }

  const getPrivateFilesFromVault = async (): Promise<any[]> => {
    if (!initializedWasm.value) await initWasm();
    let client = await connectedClient();

    // Fetch user data
    let fetchedUserData;

    try {
      fetchedUserData = await getUserData();
    } catch (err) {
      return [];
    }

    let files = [];

    for (const [privateArchiveAccess, privateArchiveName] of fetchedUserData.privateFileArchives().entries()) {
      // Get private archive
      const privateArchive = await client.getPrivateArchive(privateArchiveAccess);

      for (const [fileName, [privateFileAccess, fileMetadata]] of privateArchive.map().entries()) {
        let filePath = `${privateArchiveName.replaceAll(",", "-").replaceAll("/", "-").replaceAll(" ", "")}/${fileName}`;

        // If only one file is present in the archive, don't show the parent map
        if (privateArchive.map().size == 1) {
          filePath = fileName;
        }

        files.push({
          paths: {
            local: filePath,
          },
          size: 0,
          dateCreated: fileMetadata.created,
          dateModified: fileMetadata.modified,
          dateUploaded: fileMetadata.uploaded,
          privateDataAccess: privateFileAccess,
        });
      }
    }

    return files;
  }

  // Returns a receipt.
  const executeQuotePayments = async (quotes: [any], quotePayments: [any]) => {
    // Get the EVM network
    let evmNetwork = autonomi.getEvmNetwork();

    // Form quotes payment calldata
    const payForQuotesCalldata = autonomi.getPayForQuotesCalldata(
      evmNetwork,
      quotePayments
    );

    if (payForQuotesCalldata.approve_amount > 0) {
      state.value = "approving";

      // Form approve to spend tokens calldata
      const approveCalldata = autonomi.getApproveToSpendTokensCalldata(
        evmNetwork,
        payForQuotesCalldata.approve_spender,
        payForQuotesCalldata.approve_amount
      );

      // Approve to spend tokens
      let hash = await sendTransaction(config, {
        to: approveCalldata[1],
        data: approveCalldata[0]
      });

      // Wait for approve tx to confirm
      await waitForTransactionReceipt(config, {
        hash
      });
    }

    state.value = "paying";

    let payments = {};

    // Execute batched quote payment transactions
    for (const [calldata, quoteHashes] of payForQuotesCalldata.batched_calldata_map) {
      let hash = await sendTransaction(config, {
        to: payForQuotesCalldata.to,
        data: calldata
      });

      await waitForTransactionReceipt(config, {
        hash
      });

      // Record the transaction hashes for each quote
      quoteHashes.forEach(quoteHash => {
        payments[quoteHash] = hash;
      });
    }

    // Generate receipt
    return autonomi.getReceiptFromQuotesAndPayments(quotes, payments);
  }

  return {
    fileBeingUploaded,
    filesUploaded,
    uploadStep,
    state,
    connectedClient,
    initWasm,
    encryptData,
    getQuotesForContentAddrs,
    putPrivateData,
    putFilesToVault,
    getData,
    getPrivateData,
    getPrivateFilesFromVault,
    executeQuotePayments,
  };
});
*/
