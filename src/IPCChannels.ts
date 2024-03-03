/** Channels for renderer sending messages to main */
export enum IpcRendToMain {
  /** Save arbitrary file contents */
  saveFile = 'saveFile',
  /** Ask the main process to prompt the user for a file to save to */
  saveAsDialog = 'saveAsDialog',
  /** Set the title of the electron window */
  setWindowTitle = 'setWindowTitle',
  /** Send contents of the preview stat reports */
  ShowInAppStatReport = 'ShowInAppStatReport',
  /** Retrieve the directory containing the in-app stat report */
  GetAppDataStatReportPath = 'GetAppDataStatReportPath',
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
  /** Start a blank tournament with no file */
  newTournament = 'newTournament',
  /** Report that the stat report has been successfully written to file */
  GeneratedInAppStatReport = 'GeneratedInAppStatReport',
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
  IpcMainToRend.GeneratedInAppStatReport,
];
