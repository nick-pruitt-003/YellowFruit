/**
 * Custom React hooks
 */

import React, { useState, useEffect } from 'react';

interface ModalManager {
  dataChangedReactCallback: () => void;
}

/**
 * Holds onto a modal manager and re-renders the calling component whenever that manager's data changes
 * @param getManager Reads the manager off the tournament. Only called on the first render.
 * @returns The modal manager, to pass to its context provider
 */
export function useModalManager<T extends ModalManager>(getManager: () => T): T {
  const [, setUpdateNeeded] = useState({}); // set this object to a new object whenever we want to force a re-render
  const [mgr] = useState(getManager);
  useEffect(() => {
    mgr.dataChangedReactCallback = () => {
      setUpdateNeeded({});
    };
  }, [mgr]);
  return mgr;
}

/**
 * Bundles useState and useEffect to synchronize a component's state with the data model
 * @param valFromDataModel Value from the tournament data to synchronize with
 * @returns A value-setter tuple, like useState
 */
function useSubscription<T>(valFromDataModel: T): [T, React.Dispatch<T>] {
  const [val, setVal] = useState(valFromDataModel);
  // eslint-disable-next-line @eslint-react/set-state-in-effect
  useEffect(() => setVal(valFromDataModel), [valFromDataModel]);
  return [val, setVal];
}

export default useSubscription;
