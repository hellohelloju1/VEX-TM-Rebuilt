import collections
def calc_match_score(match_score):
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    score = (match_score.scoreTypes['connected_pins'] * 1
             + match_score.scoreTypes['connected_beams'] * 10
             + match_score.scoreTypes['color_2_stacks'] * 5
             + match_score.scoreTypes['color_3_stacks'] * 15
             + match_score.scoreTypes['stacks_in_matching_goal'] * 10
             + match_score.scoreTypes['stacks_connected_to_standoff'] * 10
             + match_score.scoreTypes['cleared_starting_pins'] * 2
             + match_score.scoreTypes['robots_contacting_scoring_objects'] * 2
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
    num_stacks = match_score.scoreTypes['color_2_stacks'] + match_score.scoreTypes['color_3_stacks']
    if num_stacks > 21:
        invalid_inputs.update(['color_2_stacks', 'color_3_stacks'])
    if num_stacks > ((match_score.scoreTypes['connected_pins'] + (match_score.scoreTypes['connected_beams'] * 3)) / 2):
        invalid_inputs.update(['connected_pins', 'connected_beams', 'color_2_stacks', 'color_3_stacks'])
    return list(invalid_inputs) if invalid_inputs else None
def calc_skills_score(skills_id, match_score):
    FinalScore = collections.namedtuple('FinalScore', ['alliances'])
    Alliance = collections.namedtuple('Alliance', ['score', 'dq', 'teams'])
    Team = collections.namedtuple('Team', ['score', 'wp', 'awp', 'ap', 'sp', 'win', 'loss', 'tie', 'no_show'])
    final = FinalScore(alliances=[])
    for alliance in match_score.alliances:
        alliance_score = (alliance.scoreTypes['connected_pins'] * 1
                          + alliance.scoreTypes['connected_beams'] * 10
                          + alliance.scoreTypes['color_2_stacks'] * 5
                          + alliance.scoreTypes['color_3_stacks'] * 15
                          + alliance.scoreTypes['stacks_in_matching_goal'] * 10
                          + alliance.scoreTypes['stacks_connected_to_standoff'] * 10
                          + alliance.scoreTypes['cleared_starting_pins'] * 2
                          + alliance.scoreTypes['robots_contacting_scoring_objects'] * 2
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
    num_stacks = get('color_2_stacks') + get('color_3_stacks')
    if num_stacks > 21:
        invalid_inputs.update(['color_2_stacks', 'color_3_stacks'])
    if num_stacks > ((get('connected_pins') + (get('connected_beams') * 3)) / 2):
        invalid_inputs.update(['connected_pins', 'connected_beams', 'color_2_stacks', 'color_3_stacks'])
    return list(invalid_inputs) if invalid_inputs else None
def get_score_breakdown(match_score):
    Parameter = collections.namedtuple('Parameter', ['name', 'scores'])
    scores = [
        Parameter(name="Connected Pins",
                  scores=[match_score.scoreTypes.get('connected_pins', default=0)]
                  ),
        Parameter(name="Connected Beams",
                  scores=[match_score.scoreTypes.get('connected_beams', default=0)]
                  ),
        Parameter(name="2-Color Stacks",
                  scores=[match_score.scoreTypes.get('color_2_stacks', default=0)]
                  ),
        Parameter(name="3-Color Stacks",
                  scores=[match_score.scoreTypes.get('color_3_stacks', default=0)]
                  ),
        Parameter(name="Cleared Starting Pins",
                  scores=[match_score.scoreTypes.get('cleared_starting_pins', default=0)]
                  ),
        Parameter(name="Robots Contacting Scoring Objects",
                  scores=[match_score.scoreTypes.get('robots_contacting_scoring_objects', default=0)]
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
