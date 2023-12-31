/** Channels for renderer sending messages to main */
export enum IpcRendToMain {
  /** Save arbitrary file contents */
  saveFile = 'saveFile',
  /** Set the title of the electron window */
  setWindowTitle = 'setWindowTitle',
}

/** Channels for main sending messages to renderer */
export enum IpcMainToRend {
  openYftFile = 'openYftFile',
  /** Request the renderer to save the currently open tournament to yft */
  saveCurrentTournament = 'saveCurrentTournament',
  /** Tell the renderer that the .yft file was saved */
  tournamentSavedSuccessfully = 'tournamentSavedSuccessfully',
  /** "Save as" menu option */
  saveAsCommand = 'saveAsYft',
  newTournament = 'newTournament',
}

/** Channels for both directions renderer<-->main */
export enum IpcBidirectional {
  ipcExample = 'ipc-example',
}

export type IpcChannels = IpcRendToMain | IpcMainToRend | IpcBidirectional;

export const rendererListenableEvents = [
  IpcMainToRend.openYftFile,
  IpcMainToRend.saveCurrentTournament,
  IpcMainToRend.tournamentSavedSuccessfully,
  IpcMainToRend.saveAsCommand,
  IpcMainToRend.newTournament,
];
