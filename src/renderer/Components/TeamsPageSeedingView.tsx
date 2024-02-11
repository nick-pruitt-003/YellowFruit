/* eslint-disable react/no-array-index-key */
import Grid from '@mui/material/Unstable_Grid2';
import { useContext, useState } from 'react';
import {
  Box,
  Divider,
  IconButton,
  List,
  ListItem,
  ListItemText,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
} from '@mui/material';
import { ArrowDropDown, ArrowDropUp } from '@mui/icons-material';
import YfCard from './YfCard';
import { TournamentContext } from '../TournamentManager';
import useSubscription from '../Utils/CustomHooks';
import { Team } from '../DataModel/Team';

export default function SeedingView() {
  return (
    <Grid container spacing={2}>
      <Grid xs={12} sm={6} md={4}>
        <SeedList />
      </Grid>
      <Grid xs={12} sm={6} md={8}>
        <YfCard title="Pools">
          <PoolView />
        </YfCard>
      </Grid>
    </Grid>
  );
}

function SeedList() {
  const tournManager = useContext(TournamentContext);
  const thisTournament = tournManager.tournament;
  const [seedList] = useSubscription(thisTournament.seeds);
  const [expectedNumTeams] = useSubscription(thisTournament.getExpectedNumberOfTeams());

  const listItems = seedList.map((tm, idx) => (
    <SeedListItem
      key={idx + 1}
      team={tm}
      seedNo={idx + 1}
      canMoveUp={idx > 0}
      canMoveDown={idx + 1 < seedList.length}
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

  const str = team === null ? '' : team.name;
  const ptrEventsForInteractiveChld = beingDraggedOn ? 'none' : 'auto';

  return (
    <div
      className="drop-target"
      draggable={team !== null}
      onDragStart={(e) => e.dataTransfer.setData(seedListItemDragKey, seedNo.toString())}
      onDragEnter={(e) => {
        e.preventDefault();
        setIsBeingDraggedOn(true);
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
          className={team !== null ? 'yf-draggable' : undefined}
          sx={{ pointerEvents: ptrEventsForInteractiveChld }}
        >{`${seedNo}. ${str}`}</ListItemText>
      </ListItem>
    </div>
  );
}

function PoolView() {
  const tournManager = useContext(TournamentContext);
  const thisTournament = tournManager.tournament;
  const [seedList] = useSubscription(thisTournament.seeds);
  const [phase] = useSubscription(thisTournament.getPrelimPhase());

  if (!phase) return null;

  return (
    <Grid container spacing={2}>
      {phase.pools.map((pool) => (
        <Grid key={pool.name} xs={12} md={6}>
          <TableContainer sx={{ border: 1, borderRadius: 1, borderColor: 'lightgray' }}>
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell sx={{ width: '40px' }} />
                  <TableCell>{pool.name}</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {pool.seeds.map((seedNo) => (
                  <PoolViewTableRow key={seedNo} seedNo={seedNo} team={seedList[seedNo - 1] || null} />
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        </Grid>
      ))}
    </Grid>
  );
}

interface IPoolViewTableRowProps {
  seedNo: number;
  team: Team | null;
}

const poolViewItemDragKey = 'PoolSeedItem';

function PoolViewTableRow(props: IPoolViewTableRowProps) {
  const { seedNo, team } = props;
  const tournManager = useContext(TournamentContext);

  return (
    <TableRow
      className={team !== null ? 'yf-draggable' : undefined}
      draggable={team !== null}
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
