/* eslint-disable react/no-array-index-key */
import Grid from '@mui/material/Unstable_Grid2';
import { useContext, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  FormControl,
  FormControlLabel,
  IconButton,
  List,
  ListItem,
  ListItemText,
  Radio,
  RadioGroup,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
} from '@mui/material';
import { ArrowDropDown, ArrowDropUp, Edit, Error, Lock } from '@mui/icons-material';
import { useHotkeys } from 'react-hotkeys-hook';
import YfCard from './YfCard';
import { TournamentContext } from '../TournamentManager';
import useSubscription from '../Utils/CustomHooks';
import { Team } from '../DataModel/Team';
import { hotkeyFormat, YfCssClasses } from '../Utils/GeneralReactUtils';
import { Pool } from '../DataModel/Pool';
import Tournament from '../DataModel/Tournament';

export default function SeedingView() {
  const tournManager = useContext(TournamentContext);
  const [readOnly] = useSubscription(tournManager.tournament.hasMatchData);
  const [usingTemplate] = useSubscription(tournManager.tournament.usingScheduleTemplate);

  return (
    <>
      <Grid container spacing={2}>
        {readOnly && (
          <Grid xs={12}>
            <Alert variant="filled" severity="info" icon={<Lock fontSize="small" />}>
              Seeds are read-only
            </Alert>
          </Grid>
        )}
        <Grid xs={12} sm={6} md={4}>
          {usingTemplate && <SeedList />}
        </Grid>
        <Grid xs={12} sm={usingTemplate ? 6 : undefined} md={usingTemplate ? 8 : undefined}>
          <YfCard title="Pools">
            <PoolView />
          </YfCard>
        </Grid>
      </Grid>
      <PoolAssignmentDialog />
    </>
  );
}

function SeedList() {
  const tournManager = useContext(TournamentContext);
  const thisTournament = tournManager.tournament;
  const [seedList] = useSubscription(thisTournament.seeds);
  const [expectedNumTeams] = useSubscription(thisTournament.getExpectedNumberOfTeams());
  const [readOnly] = useSubscription(thisTournament.hasMatchData);

  const listItems = seedList.map((tm, idx) => (
    <SeedListItem
      key={idx + 1}
      team={tm}
      seedNo={idx + 1}
      canMoveUp={idx > 0 && !readOnly}
      canMoveDown={idx + 1 < seedList.length && !readOnly}
    />
  ));
  if (expectedNumTeams !== null) {
    for (let i = seedList.length; i < expectedNumTeams || 0; i++) {
      listItems.push(<SeedListItem key={i + 1} team={null} seedNo={i + 1} canMoveUp={false} canMoveDown={false} />);
    }
  }

  return (
    <YfCard title="Seeds">
      {seedList.length > 0 && (
        <Box sx={{ marginTop: 1, border: 1, borderRadius: 1, borderColor: 'lightgray' }}>
          <List dense sx={{ py: 0 }}>
            {listItems}
          </List>
        </Box>
      )}
    </YfCard>
  );
}

interface ISeedListItemProps {
  team: Team | null;
  /** 1-indexed seed number */
  seedNo: number;
  canMoveUp: boolean;
  canMoveDown: boolean;
}

const seedListItemDragKey = 'SeedListItem';

function SeedListItem(props: ISeedListItemProps) {
  const { team, seedNo, canMoveUp, canMoveDown } = props;
  const tournManager = useContext(TournamentContext);
  const [beingDraggedOn, setIsBeingDraggedOn] = useState(false);

  const canDrag = canMoveUp || canMoveDown;
  const str = team === null ? '' : team.name;
  const ptrEventsForInteractiveChld = beingDraggedOn ? 'none' : 'auto';

  return (
    <div
      className={YfCssClasses.DropTarget}
      draggable={canDrag}
      onDragStart={(e) => e.dataTransfer.setData(seedListItemDragKey, seedNo.toString())}
      onDragEnter={(e) => {
        e.preventDefault();
        if (team !== null) setIsBeingDraggedOn(true);
      }}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        e.preventDefault();
        setIsBeingDraggedOn(false);
        if (team === null) return;
        tournManager.seedListDragDrop(e.dataTransfer.getData(seedListItemDragKey), seedNo);
      }}
      onDragLeave={(e) => {
        e.preventDefault();
        setIsBeingDraggedOn(false);
      }}
    >
      {(seedNo > 1 || beingDraggedOn) && <Divider sx={{ borderBottomWidth: beingDraggedOn ? 'thick' : 'thin' }} />}
      <ListItem
        secondaryAction={
          <>
            <IconButton size="small" disabled={!canMoveUp} onClick={() => tournManager.shiftSeedUp(seedNo)}>
              <ArrowDropUp
                sx={{ color: !canMoveUp ? 'transparent' : undefined, pointerEvents: ptrEventsForInteractiveChld }}
              />
            </IconButton>
            <IconButton size="small" disabled={!canMoveDown} onClick={() => tournManager.shiftSeedDown(seedNo)}>
              <ArrowDropDown
                sx={{ color: !canMoveDown ? 'transparent' : undefined, pointerEvents: ptrEventsForInteractiveChld }}
              />
            </IconButton>
          </>
        }
      >
        <ListItemText
          className={canDrag ? YfCssClasses.Draggable : undefined}
          sx={{ pointerEvents: ptrEventsForInteractiveChld }}
        >{`${seedNo}. ${str}`}</ListItemText>
      </ListItem>
    </div>
  );
}

function PoolView() {
  const tournManager = useContext(TournamentContext);
  const thisTournament = tournManager.tournament;
  const [phase] = useSubscription(thisTournament.getPrelimPhase());
  const [usingTemplate] = useSubscription(thisTournament.usingScheduleTemplate);

  if (!phase) return null;

  return (
    <Grid container spacing={2}>
      {phase.pools.map((pool) => (
        <Grid key={pool.name} xs={12} md={6}>
          <TableContainer sx={{ border: 1, borderRadius: 1, borderColor: 'lightgray' }}>
            {usingTemplate ? <PoolViewSeedTable pool={pool} /> : <UnseededPoolTable pool={pool} />}
          </TableContainer>
        </Grid>
      ))}
    </Grid>
  );
}

interface IPoolViewSeedTableProps {
  pool: Pool;
}

function PoolViewSeedTable(props: IPoolViewSeedTableProps) {
  const { pool } = props;
  const tournManager = useContext(TournamentContext);
  const thisTournament = tournManager.tournament;
  const [seedList] = useSubscription(thisTournament.seeds);
  const [readOnly] = useSubscription(thisTournament.hasMatchData);

  return (
    <Table size="small">
      <TableHead>
        <TableRow>
          <TableCell sx={{ width: '40px' }} />
          <TableCell>{pool.name}</TableCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {pool.seeds.map((seedNo) => (
          <PoolViewTableRow key={seedNo} seedNo={seedNo} team={seedList[seedNo - 1] || null} canDrag={!readOnly} />
        ))}
      </TableBody>
    </Table>
  );
}

interface IPoolViewTableRowProps {
  seedNo: number;
  team: Team | null;
  canDrag: boolean;
}

const poolViewItemDragKey = 'PoolSeedItem';

function PoolViewTableRow(props: IPoolViewTableRowProps) {
  const { seedNo, team, canDrag } = props;
  const tournManager = useContext(TournamentContext);

  return (
    <TableRow
      className={canDrag ? YfCssClasses.Draggable : undefined}
      draggable={canDrag}
      onDragStart={(e) => e.dataTransfer.setData(poolViewItemDragKey, seedNo.toString())}
      onDragEnter={(e) => e.preventDefault()}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        e.preventDefault();
        if (team === null) return;
        tournManager.swapSeeds(e.dataTransfer.getData(poolViewItemDragKey), seedNo);
      }}
      onDragLeave={(e) => {
        e.preventDefault();
      }}
    >
      <TableCell>{seedNo}</TableCell>
      <TableCell>{team?.name}</TableCell>
    </TableRow>
  );
}

interface IUnseededPoolTableProps {
  pool: Pool;
}

function UnseededPoolTable(props: IUnseededPoolTableProps) {
  const { pool } = props;
  const tournManager = useContext(TournamentContext);
  const [readOnly] = useSubscription(tournManager.tournament.hasMatchData);

  const listItems = pool.poolTeams.map((pt, idx) => (
    <PoolViewTableRowUnseeded
      key={pt.team.name}
      team={pt.team}
      index={idx < pool.size ? idx + 1 : null}
      pool={pool}
      canMove={!readOnly}
    />
  ));
  for (let i = listItems.length; i < pool.size; i++) {
    listItems.push(
      <PoolViewTableRowUnseeded key={`Empty ${i + 1}`} team={null} index={i + 1} pool={pool} canMove={false} />,
    );
  }

  return (
    <Table size="small">
      <TableHead>
        <TableRow>
          <TableCell sx={{ width: '40px' }} />
          <TableCell>
            {pool.name}
            {pool.sizeValidationError && (
              <Tooltip sx={{ mx: 1, verticalAlign: 'text-bottom' }} title={pool.sizeValidationError}>
                <Error fontSize="small" color="error" />
              </Tooltip>
            )}
          </TableCell>
          <TableCell sx={{ width: '40px' }} />
        </TableRow>
      </TableHead>
      <TableBody sx={{ '& .MuiSvgIcon-root': { fontSize: '1.2rem' }, '& .MuiIconButton-root': { p: 0 } }}>
        {listItems}
      </TableBody>
    </Table>
  );
}

interface IPoolViewTableRowUnseededProps {
  team: Team | null;
  index: number | null;
  pool: Pool;
  canMove: boolean;
}

const unseededTeamDragItemKey = 'SeedListItem';

function PoolViewTableRowUnseeded(props: IPoolViewTableRowUnseededProps) {
  const { team, index, pool, canMove } = props;
  const tournManager = useContext(TournamentContext);
  const dragData = unseededDragDataSerialize(pool, team);

  const handleDrop = (droppedData: string) => {
    const [originPool, draggedTeam] = unseededDragDataDeserialize(droppedData, tournManager.tournament);
    if (!originPool || !draggedTeam) return;

    tournManager.unseededTeamDragDrop(originPool, pool, draggedTeam);
  };

  return (
    <TableRow
      className={canMove ? YfCssClasses.Draggable : undefined}
      draggable={canMove}
      onDragStart={(e) => e.dataTransfer.setData(unseededTeamDragItemKey, dragData)}
      onDragEnter={(e) => e.preventDefault()}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        e.preventDefault();
        const data = e.dataTransfer.getData(unseededTeamDragItemKey);
        handleDrop(data);
      }}
    >
      <TableCell>{index}</TableCell>
      <TableCell>{team?.name ?? '-'}</TableCell>
      <TableCell>
        {canMove && team && (
          <Tooltip title="Change pool assignment">
            <IconButton size="small" onClick={() => tournManager.openPoolAssignmentModal(team, pool)}>
              <Edit />
            </IconButton>
          </Tooltip>
        )}
      </TableCell>
    </TableRow>
  );
}

function unseededDragDataSerialize(pool: Pool, team: Team | null) {
  return `${pool.name}${String.fromCharCode(1)}${team?.name || ''}`;
}

function unseededDragDataDeserialize(data: string, tourn: Tournament): [Pool | undefined, Team | undefined] {
  const [poolName, teamName] = data.split(String.fromCharCode(1));
  const pool = tourn.findPoolByName(poolName);
  const team = teamName ? tourn.findTeamByName(teamName) : undefined;
  return [pool, team];
}

function PoolAssignmentDialog() {
  const tournManager = useContext(TournamentContext);
  const modalManager = tournManager.poolAssignmentModalManager;
  const [isOpen] = useSubscription(modalManager.modalIsOpen);
  const [team] = useSubscription(modalManager.teamBeingAssigned);
  const [selectedPool, setSeletedPool] = useSubscription(modalManager.originalPoolAssigned?.name || '');

  const handleAccept = () => {
    tournManager.closePoolAssignmentModal(true);
  };
  const handleCancel = () => {
    tournManager.closePoolAssignmentModal(false);
  };
  const handleOptionChange = (option: string) => {
    modalManager.setSelectedPool(option);
    setSeletedPool(option);
  };

  useHotkeys('alt+c', () => handleCancel(), { enabled: isOpen, enableOnFormTags: true });
  useHotkeys('alt+a', () => handleAccept(), { enabled: isOpen, enableOnFormTags: true });

  if (!modalManager.phase) return null;

  return (
    <Dialog open={isOpen} fullWidth maxWidth="sm" onClose={handleCancel}>
      <DialogTitle>{`Assign ${team?.name || ''}`}</DialogTitle>
      <DialogContent>
        <FormControl>
          <RadioGroup value={selectedPool} onChange={(e) => handleOptionChange(e.target.value)}>
            {modalManager.phase.pools.map((p) => (
              <FormControlLabel key={p.name} value={p.name} label={p.name} control={<Radio />} />
            ))}
          </RadioGroup>
        </FormControl>
      </DialogContent>
      <DialogActions>
        <Button variant="outlined" onClick={handleCancel}>
          {hotkeyFormat('&Cancel')}
        </Button>
        <Button variant="outlined" onClick={handleAccept}>
          {hotkeyFormat('&Accept')}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
