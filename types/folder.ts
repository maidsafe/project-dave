export interface IFolder {
  name: string;
  parent: IFolder;
  children: IFolder[];
}

export interface IFile {
  file_access: {
    Private: any[];
  }
  metadata: {
    uploaded: number;
    created: number;
    modified: number;
    size: number;
  }
  path: string;
}

/*
 DEV Sample of IFile
{
  file_access: {Private: BYTES ARRAY}
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
