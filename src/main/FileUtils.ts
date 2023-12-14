import { BrowserWindow, IpcMainEvent, dialog } from 'electron';
import fs from 'fs';
import IpcChannels from '../IPCChannels';

export function openYftFile(mainWindow: BrowserWindow) {
  const fileNameAry = dialog.showOpenDialogSync(mainWindow, {
    filters: [{ name: 'YellowFruit Tournament', extensions: ['yft'] }],
  });
  if (fileNameAry) {
    mainWindow.webContents.send(IpcChannels.openYftFile, fileNameAry[0]);
  }
}

export function requestToSaveYftFile(mainWindow: BrowserWindow) {
  mainWindow.webContents.send(IpcChannels.saveCurrentTournament);
}

export function handleSaveFile(event: IpcMainEvent, filePath: string, fileContents: string) {
  const window = BrowserWindow.fromWebContents(event.sender);
  if (!window) return;

  fs.writeFile(filePath, fileContents, { encoding: 'utf8' }, (err) => {
    if (err) {
      dialog.showMessageBoxSync(window, { message: `Error saving file: \n\n ${err.message}` });
      return;
    }
    window.webContents.send(IpcChannels.tournamentSavedSuccessfully);
  });
}
