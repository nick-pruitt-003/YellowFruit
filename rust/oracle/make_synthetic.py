"""Make the synthetic-*.yft samples: variants of rust/samples/yft-overtime.yft that switch on the settings
no real file uses (lightning rounds, bouncebacks, finals and tiebreaker stages, team attributes, end date...).

Usage: python3 rust/oracle/make_synthetic.py <out.yft> <full|final|nobonus|template>
"""
import json, copy, sys
import os
# built from the anonymized yft-overtime sample
src = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'samples', 'yft-overtime.yft')
out = sys.argv[1]

def load():
    return json.load(open(src))

mid = [5000]
def newid():
    mid[0] += 1
    return f"Match_{mid[0]}"

def base_variant(d, *, bonuses=True, bb=True, lightning=True, flags=True, timed=False):
    t = d['objects'][0]
    y = t['YfData']
    if flags:
        y['trackSmallSchool'] = y['trackJV'] = y['trackUG'] = y['trackDiv2'] = True
    t['end_date'] = '2025-12-15T05:00:00.000Z'
    t['question_set'] = 'Synthetic Set'
    sr = t['scoring_rules']
    sr['bonuses_bounce_back'] = bb
    sr['lightning_count_per_team'] = 1 if lightning else 0
    sr['YfData']['timed'] = timed
    if not bonuses:
        for k in ['maximum_bonus_score', 'minimum_parts_per_bonus', 'maximum_parts_per_bonus', 'points_per_bonus_part', 'bonus_divisor']:
            sr.pop(k, None)
    regs = t['registrations']
    regs[0]['YfData']['isSmallSchool'] = True
    tm = regs[0]['teams'][0]
    tm['YfData'].update(isJV=True, isUG=True, isD2=True)
    years = ['-1', 'Sr.', '5th grade', 'grad', '12', 'Faculty']
    i = 0
    for r in regs:
        for team in r['teams']:
            for p in team['players']:
                p['YfData']['yearString'] = years[i % len(years)]
                p['YfData']['isUG'] = i % 3 == 0
                p['YfData']['isD2'] = i % 4 == 0
                i += 1
    ph = t['phases'][0]
    for k, rd in enumerate(ph['rounds']):
        if k % 2 == 0:
            rd['packets'] = [{'name': f'Packet {k}'}]
        for j, m in enumerate(rd['matches']):
            for s, mt in enumerate(m['match_teams']):
                if lightning:
                    mt['lightning_points'] = 10 * ((k + j + s) % 3)
                    if mt.get('points') is not None:
                        mt['points'] += mt['lightning_points']
                if bb:
                    mt['bonus_bounceback_points'] = 10 * ((k + s) % 2)
                    if mt.get('points') is not None:
                        mt['points'] += mt['bonus_bounceback_points']
    # a double forfeit, a tie and a negative score
    m = ph['rounds'][1]['matches'][0]
    for mt in m['match_teams']:
        mt['forfeit_loss'] = True
    m = ph['rounds'][2]['matches'][0]
    m['match_teams'][1]['points'] = m['match_teams'][0]['points']
    m = ph['rounds'][3]['matches'][0]
    m['match_teams'][1]['points'] = -15
    return t, ph

def add_phases(t, ph, template=False):
    teams = [pt['team']['$ref'] for pt in ph['pools'][0]['pool_teams']]
    rounds = ph['rounds']
    def clone_round(rd, number, name=None):
        r = copy.deepcopy(rd)
        r['YfData'] = {'number': number}
        if name:
            r['YfData']['nonNumericName'] = name
        r['name'] = name or str(number)
        r.pop('packets', None)
        for m in r['matches']:
            m['id'] = newid()
            m['YfData'].pop('carryoverPhases', None)
        return r
    tb = {'YfData': {'phaseType': 'Tiebreaker', 'code': '1T', 'wildCardAdvancementRules': [], 'wildCardRankingMethod': 'RankThenPPB'},
          'name': 'Phase 1 Tiebreakers', 'pools': [], 'rounds': [clone_round(rounds[0], 11, 'Phase 1 Tiebreakers')], 'id': 'Phase_Phase 1 Tiebreakers'}
    tb['rounds'][0]['matches'] = tb['rounds'][0]['matches'][:1]
    def pool(name, pos, members, carry):
        return {'YfData': {'size': len(members), 'roundRobins': 1, 'seeds': [], 'hasCarryover': carry, 'autoAdvanceRules': []},
                'name': name, 'position': pos, 'pool_teams': [{'YfData': {}, 'team': {'$ref': x}} for x in members]}
    po = {'YfData': {'phaseType': 'Playoff', 'code': '2', 'wildCardAdvancementRules': [], 'wildCardRankingMethod': 'RankThenPPB'},
          'name': 'Playoffs', 'pools': [pool('Upper', 1, teams[:3], True), pool('Lower', 2, teams[3:], True)],
          'rounds': [clone_round(rounds[4], 12), clone_round(rounds[5], 13)], 'id': 'Phase_Playoffs'}
    if template:
        po['pools'] = []
    fi = {'YfData': {'phaseType': 'Finals', 'code': 'F', 'wildCardAdvancementRules': [], 'wildCardRankingMethod': 'RankThenPPB'},
          'name': 'Finals', 'pools': [], 'rounds': [clone_round(rounds[6], 14, 'Finals')], 'id': 'Phase_Finals'}
    fi['rounds'][0]['matches'] = fi['rounds'][0]['matches'][:1]
    # carry two prelim games to the playoffs
    for rd in rounds[:2]:
        rd['matches'][1]['YfData']['carryoverPhases'] = [{'$ref': 'Phase_Playoffs'}]
    fm = fi['rounds'][0]['matches'][0]['match_teams']
    fm[1]['points'] = fm[0]['points']
    ph['pools'][0]['pool_teams'][0]['YfData']['didNotAdvance'] = True
    t['phases'] = [ph, tb, po, fi]

variant = sys.argv[2]
d = load()
if variant == 'full':
    t, ph = base_variant(d)
    add_phases(t, ph)
elif variant == 'final':
    t, ph = base_variant(d, bb=False)
    add_phases(t, ph)
    t['YfData']['finalRankingsReady'] = True
    for k, r in enumerate(t['registrations']):
        for team in r['teams']:
            team['ranks'][0]['position'] = 1 + (k % 3)
elif variant == 'nobonus':
    t, ph = base_variant(d, bonuses=False, bb=False, lightning=True, flags=False, timed=True)
elif variant == 'template':
    t, ph = base_variant(d, lightning=False)
    t['YfData']['usingScheduleTemplate'] = True
    ph['pools'][0]['YfData']['autoAdvanceRules'] = [{'tier': 1, 'ranksThatAdvance': [1, 2, 3], 'rankingRule': 'RecordthenPPGThenOther'}, {'tier': 2, 'ranksThatAdvance': [3, 4, 5, 6], 'rankingRule': 'RecordthenPPGThenOther'}]
    add_phases(t, ph, template=True)
json.dump(d, open(out, 'w'))
