export default defineEventHandler(() => {
  const files = [
    {
      paths: {
        local: "Folder 1/MyRandomFile1.txt",
        network: "safe://hnyyny1",
      },
      size: 123456,
    },
    {
      paths: {
        local: "Folder 2/Sub Folder 1/MyRandomFile2.pdf",
        network: "safe://hnyyny2",
      },
      size: 234567,
    },
    {
      paths: {
        local: "Folder 3/Sub Folder 1/Sub Folder 2/MyRandomFile3.png",
        network: "safe://hnyyny3",
      },
      size: 345678,
    },
    {
      paths: {
        local: "MyRandomFile4.zip",
        network: "safe://hnyyny4",
      },
      size: 456789,
    },
    {
      paths: {
        local:
          "Folder 5/Sub Folder 1/Sub Folder 2/Sub Folder 3/Sub Folder 4/MyRandomFile5.txt",
        network: "safe://hnyyny5",
      },
      size: 567890,
    },
    {
      paths: {
        local: "MyRandomFile6.pdf",
        network: "safe://hnyyny6",
      },
      size: 678901,
    },
    {
      paths: {
        local: "Folder 7/Sub Folder 1/MyRandomFile7.png",
        network: "safe://hnyyny7",
      },
      size: 789012,
    },
    {
      paths: {
        local: "Folder 8/Sub Folder 1/Sub Folder 2/MyRandomFile8.zip",
        network: "safe://hnyyny8",
      },
      size: 890123,
    },
    {
      paths: {
        local:
          "Folder 9/Sub Folder 1/Sub Folder 2/Sub Folder 3/MyRandomFile9.txt",
        network: "safe://hnyyny9",
      },
      size: 901234,
    },
    {
      paths: {
        local:
          "Folder 10/Sub Folder 1/Sub Folder 2/Sub Folder 3/Sub Folder 4/MyRandomFile10.pdf",
        network: "safe://hnyyny10",
      },
      size: 123456,
    },
    {
      paths: {
        local: "MyRandomFile11.png",
        network: "safe://hnyyny11",
      },
      size: 234567,
    },
    {
      paths: {
        local: "Folder 12/Sub Folder 1/MyRandomFile12.zip",
        network: "safe://hnyyny12",
      },
      size: 345678,
    },
  ];
  return files;
});
