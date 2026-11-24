import collections
def calc_match_score(match_score):
    red_alliance = 0
    blue_alliance = 1
    toggle_red = 1
    toggle_blue = 2
    auto_win_bonus = 12
    auto_tie_bonus = 6
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    red_alliance_score = 0
    blue_alliance_score = 0
    red_alliance_score += 5 * (match_score.scoreTypes['red_quad_1_red_pins']
                               + match_score.scoreTypes['red_quad_2_red_pins']
                               + match_score.scoreTypes['blue_quad_1_red_pins']
                               + match_score.scoreTypes['blue_quad_2_red_pins']
                               + match_score.scoreTypes['center_red_pins'])
    blue_alliance_score += 5 * (match_score.scoreTypes['red_quad_1_blue_pins']
                                + match_score.scoreTypes['red_quad_2_blue_pins']
                                + match_score.scoreTypes['blue_quad_1_blue_pins']
                                + match_score.scoreTypes['blue_quad_2_blue_pins']
                                + match_score.scoreTypes['center_blue_pins'])
    for quadrant in ('red_quad_1', 'red_quad_2', 'blue_quad_1', 'blue_quad_2'):
        if match_score.scoreTypes[quadrant + '_toggle'] == toggle_red:
            red_alliance_score += match_score.scoreTypes[quadrant + '_yellow_pins'] * 10
        elif match_score.scoreTypes[quadrant + '_toggle'] == toggle_blue:
            blue_alliance_score += match_score.scoreTypes[quadrant + '_yellow_pins'] * 10
    if match_score.alliances[red_alliance].scoreTypes['midfield_robots'] > match_score.alliances[blue_alliance].scoreTypes['midfield_robots']:
        red_alliance_score += match_score.scoreTypes['center_yellow_pins'] * 10
    elif match_score.alliances[red_alliance].scoreTypes['midfield_robots'] < match_score.alliances[blue_alliance].scoreTypes['midfield_robots']:
        blue_alliance_score += match_score.scoreTypes['center_yellow_pins'] * 10
    for alliance_idx, alliance in enumerate(match_score.alliances):
        alliance_score = 0
        if alliance_idx == red_alliance:
            alliance_score = red_alliance_score
        elif alliance_idx == blue_alliance:
            alliance_score = blue_alliance_score
        alliance_score += ((alliance.scoreTypes['auto'] * auto_win_bonus) +
                           (alliance.scoreTypes['auto_tie'] * auto_tie_bonus))
        alliance_score += alliance.scoreTypes['midfield_robots'] * 8
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
    def get(alliance, name):
        return match_score.alliances[alliance].scoreTypes[name]
    invalid_inputs = set()
    for quadrant in ('red_quad_1', 'red_quad_2', 'blue_quad_1', 'blue_quad_2'):
        if ((match_score.scoreTypes[quadrant + '_red_pins']
             + match_score.scoreTypes[quadrant + '_blue_pins']
             + match_score.scoreTypes[quadrant + '_yellow_pins'])
                > 16):
            invalid_inputs.update((quadrant + '_red_pins', quadrant + '_blue_pins', quadrant + '_yellow_pins'))
    if ((match_score.scoreTypes['center_red_pins']
         + match_score.scoreTypes['center_blue_pins']
         + match_score.scoreTypes['center_yellow_pins'])
            > 8):
        invalid_inputs.update(('center_red_pins', 'center_blue_pins', 'center_yellow_pins'))
    for color, max_pins in (('red', 24), ('blue', 24), ('yellow', 78)):
        if ((match_score.scoreTypes['red_quad_1_{}_pins'.format(color)]
             + match_score.scoreTypes['red_quad_2_{}_pins'.format(color)]
             + match_score.scoreTypes['blue_quad_1_{}_pins'.format(color)]
             + match_score.scoreTypes['blue_quad_2_{}_pins'.format(color)]
             + match_score.scoreTypes['center_{}_pins'.format(color)])
                > max_pins):
            invalid_inputs.update(('red_quad_1_{}_pins'.format(color), 'red_quad_2_{}_pins'.format(color),
                                   'blue_quad_1_{}_pins'.format(color), 'blue_quad_2_{}_pins'.format(color),
                                   'center_{}_pins'.format(color)))
    if (get(0, 'auto') + get(1, 'auto')) > 1:
        invalid_inputs.update(['a1_auto', 'a2_auto'])
    return list(invalid_inputs) if invalid_inputs else None
def calc_skills_score(skills_id, match_score):
    toggle_red = 1
    toggle_blue = 2
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    for alliance in match_score.alliances:
        alliance_score = 0
        alliance_score += 5 * (alliance.scoreTypes['red_quad_1_red_pins']
                               + alliance.scoreTypes['red_quad_2_red_pins']
                               + alliance.scoreTypes['blue_quad_1_blue_pins']
                               + alliance.scoreTypes['blue_quad_2_blue_pins']
                               + alliance.scoreTypes['center_red_pins']
                               + alliance.scoreTypes['center_blue_pins'])
        for n in (1, 2):
            if alliance.scoreTypes['red_quad_{}_toggle'.format(n)] == toggle_red:
                alliance_score += alliance.scoreTypes['red_quad_{}_yellow_pins'.format(n)] * 10
            if alliance.scoreTypes['blue_quad_{}_toggle'.format(n)] == toggle_blue:
                alliance_score += alliance.scoreTypes['blue_quad_{}_yellow_pins'.format(n)] * 10
        if alliance.scoreTypes['midfield_robots'] > 0:
            alliance_score += alliance.scoreTypes['center_yellow_pins'.format(n)] * 10
        alliance_score += (alliance.scoreTypes['midfield_robots'] * 8)
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
    def get(name):
        return match_score.alliances[0].scoreTypes[name]
    invalid_inputs = set()
    for quadrant in ('red_quad_1', 'red_quad_2', 'blue_quad_1', 'blue_quad_2'):
        if (get(quadrant + '_red_pins') + get(quadrant + '_blue_pins') + get(quadrant + '_yellow_pins')) > 16:
            invalid_inputs.update((quadrant + '_red_pins', quadrant + '_blue_pins', quadrant + '_yellow_pins'))
    if (get('center_red_pins') + get('center_blue_pins') + get('center_yellow_pins')) > 8:
        invalid_inputs.update(('center_red_pins', 'center_blue_pins', 'center_yellow_pins'))
    for color, max_pins in (('red', 24), ('blue', 24), ('yellow', 78)):
        if ((get('red_quad_1_{}_pins'.format(color))
             + get('red_quad_2_{}_pins'.format(color))
             + get('blue_quad_1_{}_pins'.format(color))
             + get('blue_quad_2_{}_pins'.format(color))
             + get('center_{}_pins'.format(color)))
                > max_pins):
            invalid_inputs.update(('red_quad_1_{}_pins'.format(color), 'red_quad_2_{}_pins'.format(color),
                                   'blue_quad_1_{}_pins'.format(color), 'blue_quad_2_{}_pins'.format(color),
                                   'center_{}_pins'.format(color)))
    return list(invalid_inputs) if len(invalid_inputs) > 0 else None
def get_score_breakdown(match_score):
    Parameter = collections.namedtuple('Parameter', ['name', 'scores'])
    red_alliance = 0
    blue_alliance = 1
    toggle_red = 1
    toggle_blue = 2
    auto_win_bonus = 12
    auto_tie_bonus = 6
    red_pins = (match_score.scoreTypes.get('red_quad_1_red_pins', default=0)
                + match_score.scoreTypes.get('red_quad_2_red_pins', default=0)
                + match_score.scoreTypes.get('blue_quad_1_red_pins', default=0)
                + match_score.scoreTypes.get('blue_quad_2_red_pins', default=0)
                + match_score.scoreTypes.get('center_red_pins', default=0))
    blue_pins = (match_score.scoreTypes.get('red_quad_1_blue_pins', default=0)
                 + match_score.scoreTypes.get('red_quad_2_blue_pins', default=0)
                 + match_score.scoreTypes.get('blue_quad_1_blue_pins', default=0)
                 + match_score.scoreTypes.get('blue_quad_2_blue_pins', default=0)
                 + match_score.scoreTypes.get('center_blue_pins', default=0))
    red_alliance_yellow_pins = 0
    blue_alliance_yellow_pins = 0
    for quadrant in ('red_quad_1', 'red_quad_2', 'blue_quad_1', 'blue_quad_2'):
        if match_score.scoreTypes.get(quadrant + '_toggle', default=0) == toggle_red:
            red_alliance_yellow_pins += match_score.scoreTypes.get(quadrant + '_yellow_pins', default=0)
        elif match_score.scoreTypes.get(quadrant + '_toggle', default=0) == toggle_blue:
            blue_alliance_yellow_pins += match_score.scoreTypes.get(quadrant + '_yellow_pins', default=0)
    if match_score.alliances[red_alliance].scoreTypes.get('midfield_robots', default=0) > match_score.alliances[blue_alliance].scoreTypes.get('midfield_robots', default=0):
        red_alliance_yellow_pins += match_score.scoreTypes.get('center_yellow_pins', default=0)
    elif match_score.alliances[red_alliance].scoreTypes.get('midfield_robots', default=0) < match_score.alliances[blue_alliance].scoreTypes.get('midfield_robots', default=0):
        blue_alliance_yellow_pins += match_score.scoreTypes.get('center_yellow_pins', default=0)
    a1_auto_bonus = ((match_score.alliances[0].scoreTypes.get('auto', default=0) * auto_win_bonus)
                     + (match_score.alliances[0].scoreTypes.get('auto_tie', default=0) * auto_tie_bonus))
    a2_auto_bonus = ((match_score.alliances[1].scoreTypes.get('auto', default=0) * auto_win_bonus)
                     + (match_score.alliances[1].scoreTypes.get('auto_tie', default=0) * auto_tie_bonus))
    scores = [
        Parameter(name="Alliance-colored Pins Placed",
                  scores=[red_pins, blue_pins]
                  ),
        Parameter(name="Yellow Pins Owned",
                  scores=[red_alliance_yellow_pins, blue_alliance_yellow_pins]
                  ),
        Parameter(name="Robots in Midfield",
                  scores=[match_score.alliances[0].scoreTypes.get('midfield_robots', default=0),
                          match_score.alliances[1].scoreTypes.get('midfield_robots', default=0)]
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
