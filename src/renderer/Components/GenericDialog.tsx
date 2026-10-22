import { Dialog, DialogTitle, DialogContent, DialogActions, Button } from '@mui/material';
import { useContext } from 'react';
import { useHotkeys } from 'react-hotkeys-hook';
import { GenericModalContext } from '../Modal Managers/GenericModalManager';
import { TournamentContext } from '../TournamentManager';
import useSubscription, { useModalManager } from '../Utils/CustomHooks';
import { hotkeyFormat } from '../Utils/GeneralReactUtils';

export default function GenericDialog() {
  const curTournManager = useContext(TournamentContext);
  const mgr = useModalManager(() => curTournManager.genericModalManager);

  return (
    <GenericModalContext.Provider value={mgr}>
      <GenericDialogCore />
    </GenericModalContext.Provider>
  );
}

function GenericDialogCore() {
  const modalManager = useContext(GenericModalContext);
  const [isOpen] = useSubscription(modalManager.isOpen);
  const [title] = useSubscription(modalManager.title);
  const [contents] = useSubscription(modalManager.contents);
  const [acceptButtonCaption] = useSubscription(modalManager.acceptButtonCaption);
  const [discardButtonCaption] = useSubscription(modalManager.discardAndContinueButtonCaption);
  const [cancelButtonCaption] = useSubscription(modalManager.cancelButtonCaption);

  const handleAccept = () => {
    modalManager.close(true, true);
  };

  const handleDiscard = () => {
    modalManager.close(true, false);
  };

  const handleCancel = () => {
    modalManager.close();
  };

  useHotkeys('alt+y', handleAccept, { enabled: isOpen });
  useHotkeys('alt+d', handleDiscard, { enabled: isOpen });
  useHotkeys('alt+o', handleCancel, { enabled: isOpen });

  return (
    <Dialog open={isOpen} onClose={handleCancel}>
      {title && <DialogTitle>{title}</DialogTitle>}
      <DialogContent sx={{ whiteSpace: 'pre-line' }}>{contents}</DialogContent>
      <DialogActions>
        {acceptButtonCaption && <Button onClick={handleAccept}>{hotkeyFormat(acceptButtonCaption)}</Button>}
        {discardButtonCaption && <Button onClick={handleDiscard}>{hotkeyFormat(discardButtonCaption)}</Button>}
        <Button onClick={handleCancel}>{hotkeyFormat(cancelButtonCaption)}</Button>
      </DialogActions>
    </Dialog>
  );
}
