import collections
def calc_match_score(match_score):
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    score = (match_score.scoreTypes['red_goal_floor'] * 1
             + match_score.scoreTypes['blue_goal_floor'] * 1
             + match_score.scoreTypes['red_goal_level_1'] * 3
             + match_score.scoreTypes['blue_goal_level_1'] * 3
             + match_score.scoreTypes['red_goal_level_2'] * 6
             + match_score.scoreTypes['blue_goal_level_2'] * 6
             + match_score.scoreTypes['red_goal_level_3'] * 12
             + match_score.scoreTypes['blue_goal_level_3'] * 12
             + match_score.scoreTypes['goal_level_4'] * 16
             )
    alliance_dq = False
    ignore_no_show = False
    if IsMatchTopNFinals(match_score.matchTuple):
        if any((match_score.alliances[0].teams[0].dq, match_score.alliances[1].teams[0].dq)):
            alliance_dq = True
            score = 0
        elif ((match_score.alliances[0].teams[0].noShow and not match_score.alliances[1].teams[0].noShow) or
              (match_score.alliances[1].teams[0].noShow and not match_score.alliances[0].teams[0].noShow)):
            ignore_no_show = True
    for alliance_idx, alliance in enumerate(match_score.alliances):
        dq = True if (alliance_dq or alliance.teams[0].dq or
                      (alliance.teams[0].noShow and not ignore_no_show)) else False
        final.alliances.append(Alliance(
            score=score if not dq else 0,
            dq=dq,
            teams=[],
        ))
    for alliance_idx, alliance in enumerate(final.alliances):
        for team_score in match_score.alliances[alliance_idx].teams:
            if team_score.dq or (team_score.noShow and not ignore_no_show) or alliance.dq:
                score = 0
                stop_time = 0
            else:
                score = alliance.score
                stop_time = match_score.scoreTypes['stoptime']
            alliance.teams.append(Team(
                score=score,
                wp=stop_time,
                awp=0,
                ap=0,
                sp=0,
                win=0,
                loss=0,
                tie=0,
                no_show=team_score.noShow,
            ))
    return final
def check_constraints(match_score):
    invalid_inputs = set()
    num_red = (match_score.scoreTypes['red_goal_floor'] + match_score.scoreTypes['red_goal_level_1']
               + match_score.scoreTypes['red_goal_level_2'] + match_score.scoreTypes['red_goal_level_3'])
    num_blue = (match_score.scoreTypes['blue_goal_floor'] + match_score.scoreTypes['blue_goal_level_1']
                + match_score.scoreTypes['blue_goal_level_2'] + match_score.scoreTypes['blue_goal_level_3'])
    num_yellow = match_score.scoreTypes['goal_level_4']
    if (num_red + num_yellow) > 22:
        invalid_inputs.update(['red_goal_floor', 'red_goal_level_1', 'red_goal_level_2', 'red_goal_level_3'])
        invalid_inputs.add('goal_level_4')
    if (num_blue + num_yellow) > 22:
        invalid_inputs.update(['blue_goal_floor', 'blue_goal_level_1', 'blue_goal_level_2', 'blue_goal_level_3'])
        invalid_inputs.add('goal_level_4')
    if (num_red + num_blue + num_yellow) > 38:
        invalid_inputs.update(['red_goal_floor', 'red_goal_level_1', 'red_goal_level_2', 'red_goal_level_3'])
        invalid_inputs.update(['blue_goal_floor', 'blue_goal_level_1', 'blue_goal_level_2', 'blue_goal_level_3'])
        invalid_inputs.add('goal_level_4')
    return list(invalid_inputs) if invalid_inputs else None
def calc_skills_score(skills_id, match_score):
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    for alliance in match_score.alliances:
        alliance_score = (alliance.scoreTypes['red_goal_floor'] * 1
                          + alliance.scoreTypes['blue_goal_floor'] * 1
                          + alliance.scoreTypes['red_goal_level_1'] * 3
                          + alliance.scoreTypes['blue_goal_level_1'] * 3
                          + alliance.scoreTypes['red_goal_level_2'] * 6
                          + alliance.scoreTypes['blue_goal_level_2'] * 6
                          + alliance.scoreTypes['red_goal_level_3'] * 12
                          + alliance.scoreTypes['blue_goal_level_3'] * 12
                          + alliance.scoreTypes['goal_level_4'] * 16
                          )
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
    num_red = (get('red_goal_floor') + get('red_goal_level_1') + get('red_goal_level_2') + get('red_goal_level_3'))
    num_blue = (get('blue_goal_floor') + get('blue_goal_level_1') + get('blue_goal_level_2') + get('blue_goal_level_3'))
    num_yellow = get('goal_level_4')
    if (num_red + num_yellow) > 22:
        invalid_inputs.update(['red_goal_floor', 'red_goal_level_1', 'red_goal_level_2', 'red_goal_level_3'])
        invalid_inputs.add('goal_level_4')
    if (num_blue + num_yellow) > 22:
        invalid_inputs.update(['blue_goal_floor', 'blue_goal_level_1', 'blue_goal_level_2', 'blue_goal_level_3'])
        invalid_inputs.add('goal_level_4')
    if (num_red + num_blue + num_yellow) > 38:
        invalid_inputs.update(['red_goal_floor', 'red_goal_level_1', 'red_goal_level_2', 'red_goal_level_3'])
        invalid_inputs.update(['blue_goal_floor', 'blue_goal_level_1', 'blue_goal_level_2', 'blue_goal_level_3'])
        invalid_inputs.add('goal_level_4')
    return list(invalid_inputs) if invalid_inputs else None
def get_score_breakdown(match_score):
    Parameter = collections.namedtuple('Parameter', ['name', 'scores'])
    floor = match_score.scoreTypes.get('red_goal_floor', default=0) + match_score.scoreTypes.get('blue_goal_floor', default=0)
    level_1 = match_score.scoreTypes.get('red_goal_level_1', default=0) + match_score.scoreTypes.get('blue_goal_level_1', default=0)
    level_2 = match_score.scoreTypes.get('red_goal_level_2', default=0) + match_score.scoreTypes.get('blue_goal_level_2', default=0)
    level_3 = match_score.scoreTypes.get('red_goal_level_3', default=0) + match_score.scoreTypes.get('blue_goal_level_3', default=0)
    level_4 = match_score.scoreTypes.get('goal_level_4', default=0)
    scores = [
        Parameter(name="Floor Goal Bean Bags",
                  scores=[floor]
                  ),
        Parameter(name="L1 Goal Bean Bags",
                  scores=[level_1],
                  ),
        Parameter(name="L2 Goal Bean Bags",
                  scores=[level_2],
                  ),
        Parameter(name="L3 Goal Bean Bags",
                  scores=[level_3],
                  ),
        Parameter(name="L4 Goal Bean Bags",
                  scores=[level_4],
                  ),
    ]
    stop_time = match_score.scoreTypes.get('stoptime', default=0)
    if stop_time > 0:
        scores.append(Parameter(
            name="Match Stop Time",
            scores=[
                stop_time,
            ]
        ))
    return scores
