import { vi } from 'vitest';

vi.stubGlobal('window', {
  electron: {
    ipcRenderer: {
      sendMessage: vi.fn(),
      on: vi.fn(),
      once: vi.fn(),
    },
  },
});
