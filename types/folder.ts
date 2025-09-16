export interface IFolder {
  name: string;
  parent: IFolder;
  children: IFolder[];
}

export interface IArchive {
  name: string;
  archive_access: {
    Private: string;
  } | {
    Public: string;
  };
  files: IFileMetadata[];
}

export interface IVaultStructure {
  archives: IArchive[];
  failed_archives: IFailedArchive[];
  files: IFileMetadata[];
}

export interface IFailedArchive {
  name: string;
  archive_access: {
    Private: string;
  } | {
    Public: string;
  };
}

export interface IFileMetadata {
  path: string;
  metadata: {
    uploaded: number;
    created: number;
    modified: number;
    size: number;
  };
  file_type: 'Public' | 'Private';
  is_loaded: boolean;
  archive_name: string;
  access_data?: {
    Public?: string; // Data address string
    Private?: any; // DataMapChunk bytes
  };
}

export interface IFile {
  file_access: {
    Private: any; // Bytes array containing datamap chunk
  } | {
    Public: string; // Data address string
  }
  metadata: {
    uploaded: number;
    created: number;
    modified: number;
    size: number;
  }
  path: string;
  is_loaded?: boolean;
  is_loading?: boolean;
  load_error?: boolean;
}

/*
 DEV Sample of IFile
{
  file_access: {Private: BYTES ARRAY} or {Public: "data_address_string"}
  metadata: {uploaded: 1734804991, created: 1734804991, modified: 1734804991, size: 4357964}
  path: "/ant.log"
}
*/
export interface IFileOLD {
  // TODO: DELETE THIS IF NOT USED
  paths: {
    local: string;
    network: string;
  };
  size: number;
  paprint: IFolder;
}
