import { Phase, PhaseTypes } from '../DataModel/Phase';
import { Pool } from '../DataModel/Pool';
import { Team } from '../DataModel/Team';

export default class PoolAssignmentModalManager {
  modalIsOpen: boolean = false;

  phase?: Phase;

  teamBeingAssigned?: Team;

  originalPoolAssigned?: Pool;

  selectedPool?: Pool;

  showNoneOption: boolean = false;

  acceptCallback: () => void;

  static readonly noneOptionKey = 'none-pool-radio-option';

  constructor() {
    this.acceptCallback = () => {};
  }

  reset() {
    delete this.teamBeingAssigned;
    delete this.selectedPool;
  }

  openModal(team: Team, phase: Phase, acceptCallback: () => void, originalPool?: Pool) {
    this.teamBeingAssigned = team;
    this.originalPoolAssigned = originalPool;
    this.selectedPool = originalPool;
    this.phase = phase;
    this.showNoneOption = phase.phaseType === PhaseTypes.Playoff;
    this.acceptCallback = acceptCallback;
    this.modalIsOpen = true;
  }

  closeModal(shouldSave: boolean) {
    if (shouldSave) {
      this.saveData();
    }
    this.modalIsOpen = false;
    this.reset();
  }

  saveData() {
    this.acceptCallback();
  }

  /** A function that just removes the team from the old pool and adds to the new. Available for use as an accept callback. */
  simplePoolSwitch() {
    if (!this.teamBeingAssigned) return;

    this.originalPoolAssigned?.removeTeam(this.teamBeingAssigned);
    this.selectedPool?.addTeam(this.teamBeingAssigned);
    if (this.originalPoolAssigned) this.phase?.revalidateMatchesForPoolCompatibility(this.originalPoolAssigned);
    if (this.selectedPool) this.phase?.revalidateMatchesForPoolCompatibility(this.selectedPool);
  }

  setSelectedPool(optionKey: string) {
    if (optionKey === PoolAssignmentModalManager.noneOptionKey) {
      delete this.selectedPool;
      return;
    }
    const lookedUpPool = this.phase?.findPoolByName(optionKey);
    if (!lookedUpPool) {
      // Lookup failure shouldn't happen — the dialog only offers names from
      // phase.pools — but guard against it so we don't silently wipe a valid
      // selection if a stale/unknown name slips through.
      if (process.env.NODE_ENV !== 'production') {
        console.warn(`PoolAssignmentModalManager.setSelectedPool: no pool named "${optionKey}" in phase "${this.phase?.name}"; keeping previous selection.`);
      }
      return;
    }
    this.selectedPool = lookedUpPool;
  }
}
