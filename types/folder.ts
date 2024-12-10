export interface IFolder {
  name: string;
  parent: IFolder;
  children: IFolder[];
}

export interface IFile {
  paths: {
    local: string;
    network: string;
  };
  size: number;
  paprint: IFolder;
}
