import collections
def calc_match_score(match_score):
    red_alliance = 0
    blue_alliance = 1
    control_bonus_red = 1
    control_bonus_blue = 2
    auto_win_bonus = 10
    auto_tie_bonus = 5
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    red_alliance_score = 0
    blue_alliance_score = 0
    red_alliance_score += 3 * (match_score.scoreTypes['red_blocks_long_goal_1']
                               + match_score.scoreTypes['red_blocks_long_goal_2']
                               + match_score.scoreTypes['red_blocks_upper_goal']
                               + match_score.scoreTypes['red_blocks_lower_goal'])
    blue_alliance_score += 3 * (match_score.scoreTypes['blue_blocks_long_goal_1']
                                + match_score.scoreTypes['blue_blocks_long_goal_2']
                                + match_score.scoreTypes['blue_blocks_upper_goal']
                                + match_score.scoreTypes['blue_blocks_lower_goal'])
    red_alliance_score += 10 if match_score.scoreTypes['control_bonus_long_goal_1'] == control_bonus_red else 0
    red_alliance_score += 10 if match_score.scoreTypes['control_bonus_long_goal_2'] == control_bonus_red else 0
    blue_alliance_score += 10 if match_score.scoreTypes['control_bonus_long_goal_1'] == control_bonus_blue else 0
    blue_alliance_score += 10 if match_score.scoreTypes['control_bonus_long_goal_2'] == control_bonus_blue else 0
    if match_score.scoreTypes['red_blocks_upper_goal'] > match_score.scoreTypes['blue_blocks_upper_goal']:
        red_alliance_score += 8
    elif match_score.scoreTypes['blue_blocks_upper_goal'] > match_score.scoreTypes['red_blocks_upper_goal']:
        blue_alliance_score += 8
    if match_score.scoreTypes['red_blocks_lower_goal'] > match_score.scoreTypes['blue_blocks_lower_goal']:
        red_alliance_score += 6
    elif match_score.scoreTypes['blue_blocks_lower_goal'] > match_score.scoreTypes['red_blocks_lower_goal']:
        blue_alliance_score += 6
    for alliance_idx, alliance in enumerate(match_score.alliances):
        if alliance_idx == red_alliance:
            alliance_score = red_alliance_score
        elif alliance_idx == blue_alliance:
            alliance_score = blue_alliance_score
        else:
            alliance_score = 0
        alliance_score += ((alliance.scoreTypes['auto'] * auto_win_bonus) +
                           (alliance.scoreTypes['auto_tie'] * auto_tie_bonus))
        if alliance.scoreTypes['parked_robots'] == 1:
            alliance_score += 8
        elif alliance.scoreTypes['parked_robots'] == 2:
            alliance_score += 30
        alliance_dq = False
        if IsMatchElim(match_score.matchTuple) or IsMatchRoundRobin(match_score.matchTuple):
            alliance_dq = any((True for team in alliance.teams if team.dq))
        alliance_dq |= next((0 for team in alliance.teams if (not team.dq and not team.noShow)), 1)
        final.alliances.append(Alliance(
            score=alliance_score if not alliance_dq else 0,
            dq=alliance_dq,
            teams=[],
        ))
    for alliance_idx, alliance in enumerate(final.alliances):
        for team_score in match_score.alliances[alliance_idx].teams:
            if team_score.dq or team_score.noShow or alliance.dq:
                alliance.teams.append(Team(
                    score=0,
                    wp=0,
                    awp=0,
                    ap=0,
                    sp=0,
                    win=0,
                    loss=1,
                    tie=0,
                    no_show=team_score.noShow,
                ))
                continue
            this_alliance = match_score.alliances[alliance_idx]
            other_alliance = match_score.alliances[1 - alliance_idx]
            our_dq = sum([1 if t.dq else 0 for t in this_alliance.teams])
            their_dq = sum([1 if t.dq else 0 for t in other_alliance.teams])
            wp = 0
            if final.alliances[1 - alliance_idx].dq:
                wp = 2
            else:
                if final.alliances[alliance_idx].score > final.alliances[1 - alliance_idx].score:
                    wp = 2
                elif final.alliances[alliance_idx].score < final.alliances[1 - alliance_idx].score:
                    if their_dq > 0:
                        wp = 2
                else:
                    if their_dq == 0:
                        wp = 1
                    elif their_dq > 0 and our_dq == 0:
                        wp = 2
                    elif their_dq > 0 and our_dq > 0:
                        wp = 1
            awp = this_alliance.scoreTypes['auto_wp']
            ap = (this_alliance.scoreTypes['auto'] * auto_win_bonus) +\
                 (this_alliance.scoreTypes['auto_tie'] * auto_tie_bonus)
            sp = 0
            if final.alliances[1 - alliance_idx].dq:
                sp = final.alliances[alliance_idx].score
            else:
                if final.alliances[alliance_idx].score > final.alliances[1 - alliance_idx].score:
                    sp = final.alliances[1 - alliance_idx].score
                else:
                    sp = final.alliances[alliance_idx].score
            win = int(wp == 2)
            tie = int(wp == 1)
            loss = int(not(win or tie))
            alliance.teams.append(Team(
                score=alliance.score,
                wp=wp,
                awp=awp,
                ap=ap,
                sp=sp,
                win=win,
                loss=loss,
                tie=tie,
                no_show=team_score.noShow,
            ))
    return final
def check_constraints(match_score):
    control_bonus_red = 1
    control_bonus_blue = 2
    def get(alliance, name):
        return match_score.alliances[alliance].scoreTypes[name]
    invalid_inputs = set()
    if match_score.scoreTypes['red_blocks_long_goal_1'] + match_score.scoreTypes['blue_blocks_long_goal_1'] > 16:
        invalid_inputs.update(('red_blocks_long_goal_1', 'blue_blocks_long_goal_1'))
    if match_score.scoreTypes['red_blocks_long_goal_2'] + match_score.scoreTypes['blue_blocks_long_goal_2'] > 16:
        invalid_inputs.update(('red_blocks_long_goal_2', 'blue_blocks_long_goal_2'))
    if match_score.scoreTypes['red_blocks_upper_goal'] + match_score.scoreTypes['blue_blocks_upper_goal'] > 8:
        invalid_inputs.update(('red_blocks_upper_goal', 'blue_blocks_upper_goal'))
    if match_score.scoreTypes['red_blocks_lower_goal'] + match_score.scoreTypes['blue_blocks_lower_goal'] > 8:
        invalid_inputs.update(('red_blocks_lower_goal', 'blue_blocks_lower_goal'))
    if ((match_score.scoreTypes['red_blocks_long_goal_1'] + match_score.scoreTypes['red_blocks_long_goal_2']
         + match_score.scoreTypes['red_blocks_upper_goal'] + match_score.scoreTypes['red_blocks_lower_goal']) > 44):
        invalid_inputs.update(('red_blocks_long_goal_1', 'red_blocks_long_goal_2', 'red_blocks_upper_goal',
                               'red_blocks_lower_goal'))
    if ((match_score.scoreTypes['blue_blocks_long_goal_1'] + match_score.scoreTypes['blue_blocks_long_goal_2']
         + match_score.scoreTypes['blue_blocks_upper_goal'] + match_score.scoreTypes['blue_blocks_lower_goal']) > 44):
        invalid_inputs.update(('blue_blocks_long_goal_1', 'blue_blocks_long_goal_2', 'blue_blocks_upper_goal',
                               'blue_blocks_lower_goal'))
    if ((match_score.scoreTypes['red_blocks_long_goal_1'] + match_score.scoreTypes['red_blocks_long_goal_2']
         + match_score.scoreTypes['red_blocks_upper_goal'] + match_score.scoreTypes['red_blocks_lower_goal']
         + match_score.scoreTypes['blue_blocks_long_goal_1'] + match_score.scoreTypes['blue_blocks_long_goal_2']
         + match_score.scoreTypes['blue_blocks_upper_goal'] + match_score.scoreTypes['blue_blocks_lower_goal']) > 48):
        invalid_inputs.update(('red_blocks_long_goal_1', 'red_blocks_long_goal_2', 'red_blocks_upper_goal',
                               'red_blocks_lower_goal', 'blue_blocks_long_goal_1', 'blue_blocks_long_goal_2',
                               'blue_blocks_upper_goal', 'blue_blocks_lower_goal'))
    if match_score.scoreTypes['control_bonus_long_goal_1'] == control_bonus_red:
        if match_score.scoreTypes['red_blocks_long_goal_1'] == 0:
            invalid_inputs.update(('control_bonus_long_goal_1', 'red_blocks_long_goal_1'))
    elif match_score.scoreTypes['control_bonus_long_goal_1'] == control_bonus_blue:
        if match_score.scoreTypes['blue_blocks_long_goal_1'] == 0:
            invalid_inputs.update(('control_bonus_long_goal_1', 'blue_blocks_long_goal_1'))
    if match_score.scoreTypes['control_bonus_long_goal_2'] == control_bonus_red:
        if match_score.scoreTypes['red_blocks_long_goal_2'] == 0:
            invalid_inputs.update(('control_bonus_long_goal_2', 'red_blocks_long_goal_2'))
    elif match_score.scoreTypes['control_bonus_long_goal_2'] == control_bonus_blue:
        if match_score.scoreTypes['blue_blocks_long_goal_2'] == 0:
            invalid_inputs.update(('control_bonus_long_goal_2', 'blue_blocks_long_goal_2'))
    if (get(0, 'auto') + get(1, 'auto')) > 1:
        invalid_inputs.update(['a1_auto', 'a2_auto'])
    return list(invalid_inputs) if invalid_inputs else None
def calc_skills_score(skills_id, match_score):
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    for alliance in match_score.alliances:
        alliance_score = (alliance.scoreTypes['scored_blocks'] * 1) \
                         + (alliance.scoreTypes['filled_control_zones_long_goal'] * 5) \
                         + (alliance.scoreTypes['filled_control_zones_center_goal'] * 10) \
                         + (alliance.scoreTypes['cleared_park_zones'] * 5) \
                         + (alliance.scoreTypes['cleared_loaders'] * 5) \
                         + (alliance.scoreTypes['parked_robots'] * 15)
        final.alliances.append(Alliance(
            score=alliance_score,
            dq=False,
            teams=[Team(
                score=alliance_score,
                wp=0,
                awp=0,
                ap=0,
                sp=0,
                win=0,
                loss=0,
                tie=0,
                no_show=False,
            )],
        ))
    return final
def check_skills_constraints(skills_id, match_score):
    def get(alliance, name):
        return match_score.alliances[alliance].scoreTypes[name]
    invalid_inputs = set()
    blocks_needed = ((3 * get(0, 'filled_control_zones_long_goal'))
                     + (7 * get(0, 'filled_control_zones_center_goal')))
    if blocks_needed > get(0, 'scored_blocks'):
        invalid_inputs.add('scored_blocks')
        if get(0, 'filled_control_zones_long_goal') > 0:
            invalid_inputs.add('filled_control_zones_long_goal')
        if get(0, 'filled_control_zones_center_goal') > 0:
            invalid_inputs.add('filled_control_zones_center_goal')
    return list(invalid_inputs) if len(invalid_inputs) > 0 else None
def get_score_breakdown(match_score):
    Parameter = collections.namedtuple('Parameter', ['name', 'scores'])
    auto_win_bonus = 10
    auto_tie_bonus = 5
    control_bonus_red = 1
    control_bonus_blue = 2
    red_blocks_scored = (match_score.scoreTypes.get('red_blocks_long_goal_1', default=0)
                         + match_score.scoreTypes.get('red_blocks_long_goal_2', default=0)
                         + match_score.scoreTypes.get('red_blocks_upper_goal', default=0)
                         + match_score.scoreTypes.get('red_blocks_lower_goal', default=0))
    blue_blocks_scored = (match_score.scoreTypes.get('blue_blocks_long_goal_1', default=0)
                          + match_score.scoreTypes.get('blue_blocks_long_goal_2', default=0)
                          + match_score.scoreTypes.get('blue_blocks_upper_goal', default=0)
                          + match_score.scoreTypes.get('blue_blocks_lower_goal', default=0))
    red_long_goals_controlled = 0
    blue_long_goals_controlled = 0
    red_long_goals_controlled += 1 if match_score.scoreTypes.get('control_bonus_long_goal_1', default=0) == control_bonus_red else 0
    red_long_goals_controlled += 1 if match_score.scoreTypes.get('control_bonus_long_goal_2', default=0) == control_bonus_red else 0
    blue_long_goals_controlled += 1 if match_score.scoreTypes.get('control_bonus_long_goal_1', default=0) == control_bonus_blue else 0
    blue_long_goals_controlled += 1 if match_score.scoreTypes.get('control_bonus_long_goal_2', default=0) == control_bonus_blue else 0
    red_upper_goal_control = 0
    blue_upper_goal_control = 0
    if match_score.scoreTypes.get('red_blocks_upper_goal', default=0) > match_score.scoreTypes.get('blue_blocks_upper_goal', default=0):
        red_upper_goal_control = 1
    elif match_score.scoreTypes.get('blue_blocks_upper_goal', default=0) > match_score.scoreTypes.get('red_blocks_upper_goal', default=0):
        blue_upper_goal_control = 1
    red_lower_goal_control = 0
    blue_lower_goal_control = 0
    if match_score.scoreTypes.get('red_blocks_lower_goal', default=0) > match_score.scoreTypes.get('blue_blocks_lower_goal', default=0):
        red_lower_goal_control = 1
    elif match_score.scoreTypes.get('blue_blocks_lower_goal', default=0) > match_score.scoreTypes.get('red_blocks_lower_goal', default=0):
        blue_lower_goal_control = 1
    a1_auto_bonus = ((match_score.alliances[0].scoreTypes.get('auto', default=0) * auto_win_bonus)
                     + (match_score.alliances[0].scoreTypes.get('auto_tie', default=0) * auto_tie_bonus))
    a2_auto_bonus = ((match_score.alliances[1].scoreTypes.get('auto', default=0) * auto_win_bonus)
                     + (match_score.alliances[1].scoreTypes.get('auto_tie', default=0) * auto_tie_bonus))
    scores = [
        Parameter(name="Blocks Scored",
                  scores=[red_blocks_scored, blue_blocks_scored]
                  ),
        Parameter(name="Long Goals Controlled",
                  scores=[red_long_goals_controlled, blue_long_goals_controlled]
                  ),
        Parameter(name="Upper Goal Controlled",
                  scores=[red_upper_goal_control, blue_upper_goal_control]
                  ),
        Parameter(name="Lower Goal Controlled",
                  scores=[red_lower_goal_control, blue_lower_goal_control]
                  ),
        Parameter(name="Parked Robots",
                  scores=[match_score.alliances[0].scoreTypes.get('parked_robots', default=0),
                          match_score.alliances[1].scoreTypes.get('parked_robots', default=0)]
                  ),
        Parameter(name="Autonomous Bonus",
                  scores=[a1_auto_bonus, a2_auto_bonus]
                  ),
        Parameter(name="Autonomous Win Point",
                  scores=[match_score.alliances[0].scoreTypes.get('auto_wp', default=0),
                          match_score.alliances[1].scoreTypes.get('auto_wp', default=0)]
                  ),
    ]
    return scores
