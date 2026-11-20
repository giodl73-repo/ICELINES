//! Standalone local report; unsupported individual box-score statistics stay absent.
use super::*;
pub(super) fn e(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub fn html(r: &ResultSet) -> String {
    let mut s = String::from(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rinkcraft tournament</title><style>
    :root{color-scheme:light;--ink:#112c3b;--line:#b8cad1;--ice:#eef5f7;--accent:#006968}*{box-sizing:border-box}body{margin:0;background:var(--ice);color:var(--ink);font:15px/1.45 system-ui,sans-serif}main{max-width:1140px;margin:auto;padding:24px}header{border-top:8px solid var(--accent);border-bottom:3px solid var(--ink);padding:16px 0}h1{font-size:30px;margin:4px 0}h2{font-size:19px;margin:22px 0 8px}p{margin:7px 0}small,.muted{color:#45616d}section{min-width:0;background:white;border:1px solid var(--line);padding:16px;margin:16px 0}.grid{display:grid;grid-template-columns:minmax(0,1.3fr) minmax(0,1fr);gap:16px}.grid section{margin:0}table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}th,td{text-align:left;padding:7px 8px;border-bottom:1px solid #dbe5e9}th{font-size:12px;text-transform:uppercase;letter-spacing:.03em}strong.score{font-size:19px;white-space:nowrap}.winner{font-size:25px;font-weight:750;color:var(--accent)}.tag{font-size:12px;background:#d9edeb;padding:3px 7px;border-radius:4px}.bar{height:6px;background:#dce9ec;margin:3px 0 9px}.bar span{display:block;height:100%;background:var(--accent)}details{border-top:1px solid var(--line);padding-top:8px;margin-top:12px}summary{cursor:pointer;font-weight:650}pre{white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px}a{color:#006968}.scroll{overflow-x:auto}.attributes{font-size:12px}footer{margin:20px 0;font-size:12px;overflow-wrap:anywhere}@media(max-width:740px){main{padding:12px}.grid{grid-template-columns:minmax(0,1fr)}h1{font-size:26px}th,td{padding:6px}.attributes{min-width:700px}}
    </style><main><header><span class="tag">RINKCRAFT · FICTIONAL PLAYTEST</span><h1>The first tournament</h1>"#,
    );
    s.push_str(&format!("<p>{} teams · {} rostered characters · {} games · seed {}</p><p class=muted>Abstract full-game scores. Versioned task model; continuous possessions and individual box scores are future work.</p></header>",r.active_teams.len(),r.active_teams.iter().map(|t|t.roster.members.len()).sum::<usize>(),r.illustrative.games.len(),r.illustrative.seed));
    s.push_str(&format!("<section><small>ILLUSTRATIVE TOURNAMENT CHAMPION</small><div class=winner>{}</div><p>This is one seeded run. The title-frequency batch below describes repeated draws under the same authored model.</p></section>",e(&r.illustrative.champion)));
    s.push_str("<div class=grid><section><h2>Round-robin standings</h2><div class=scroll><table><tr><th>Team</th><th>GP</th><th>W</th><th>RW</th><th>L</th><th>OTL</th><th>GF–GA</th><th>Pts</th></tr>");
    for t in &r.illustrative.standings {
        s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}–{}</td><td><b>{}</b></td></tr>",e(&t.team),t.games,t.wins,t.regulation_wins,t.regulation_losses,t.overtime_losses,t.goals_for,t.goals_against,t.points));
    }
    s.push_str("</table></div><p><small>The final is excluded from these standings. Win 2 points; tied-regulation loss 1.</small></p></section><section><h2>Title frequency</h2>");
    s.push_str(&format!("<p><small>{} tournament seeds, starting at {}. Sampling uncertainty only; this is not a calibrated NHL forecast or proof of balance.</small></p>",r.request.batch_trials,r.request.seed));
    for t in &r.title_frequencies {
        s.push_str(&format!("<p><b>{}</b> · {:.2}% <small>({} titles; MC SE {:.2} percentage points)</small></p><div class=bar><span style=\"width:{:.4}%\"></span></div>",e(&t.team),100.0*t.frequency,t.titles,100.0*t.monte_carlo_standard_error,100.0*t.frequency));
    }
    s.push_str("</section></div><section><h2>Game results</h2><div class=scroll><table><tr><th>Game</th><th>Matchup</th><th>Score</th><th>Resolution</th><th>Model goal means</th></tr>");
    for g in &r.illustrative.games {
        let m = &r.matchups[g.matchup_index];
        s.push_str(&format!("<tr><td>{}</td><td>{} — {}</td><td><strong class=score>{}–{}</strong></td><td>{}</td><td>{:.3} / {:.3}{}</td></tr>",e(&g.phase),e(&g.home),e(&g.away),g.home_goals,g.away_goals,if g.resolution=="regulation"{"Regulation"}else{"Abstract deciding goal"},m.home_model_goals_mean,m.away_model_goals_mean,if m.rate_cap_applied{" · capped"}else{""}));
    }
    s.push_str("</table></div><p><small>A tied regulation score receives one modeled deciding goal. No overtime event sequence or shootout has been simulated.</small></p></section><section><h2>Teams and character profiles</h2><p>Numbers below are authored 1–20 game attributes. Deployment weights are declared opportunity mixtures, not observed shifts or actual TOI.</p>");
    for t in &r.active_teams {
        s.push_str(&format!("<details><summary>{} · {} characters · starter {}</summary><p>{} active joint units, conditioned on this starter. Bench players remain in the roster.</p><div class=scroll><table class=attributes><tr><th>Character</th><th>Position</th><th>Class</th><th>Attributes</th><th>Tasks available in this model</th></tr>",e(&t.roster.team),t.roster.members.len(),e(&t.starter_id),t.roster.units.len()));
        for member in &t.roster.members {
            let attrs = member
                .character
                .capabilities
                .iter()
                .map(|c| format!("{} {}", e(&c.label), c.score))
                .collect::<Vec<_>>()
                .join(" · ");
            let tasks: BTreeSet<_> = r
                .matchups
                .iter()
                .flat_map(|m| &m.task_opportunities)
                .filter(|x| x.member_id == member.id)
                .map(|x| format!("{} ({})", x.task, x.capability))
                .collect();
            s.push_str(&format!("<tr><td>{}<br><small>{}</small></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",e(&member.character.name),e(&member.id),e(&member.character.position),e(&member.character.class_id),attrs,e(&if tasks.is_empty(){"No task in this declared deployment".into()}else{tasks.into_iter().collect::<Vec<_>>().join(" · ")})));
        }
        s.push_str("</table></div></details>");
    }
    s.push_str("</section><section><h2>Rules and limits</h2><ul>");
    for line in r.tournament_rules.iter().chain(&r.limits) {
        s.push_str(&format!("<li>{}</li>", e(line)));
    }
    s.push_str("</ul><details><summary>Matchup evidence and expected task checks</summary><p>Expected checks per modeled attack window, not actual player game statistics.</p><pre>");
    s.push_str(&e(&serde_json::to_string_pretty(&r.matchups).unwrap()));
    s.push_str(&format!("</pre></details></section><footer>Model {} · input SHA-256 {} · <a href=\"tournament.json\">Full result JSON</a> · <a href=\"request.json\">Canonical character/deployment input</a></footer></main></html>",e(&r.model_id),e(&r.input_sha256)));
    if r.request.tactics.is_some() {
        s = s.replacen("The first tournament", "Rules 0.2 tournament", 1);
    }
    s
}
#[cfg(test)]
mod tests {
    #[test]
    fn user_text_is_escaped() {
        assert_eq!(super::e("<&\"'>"), "&lt;&amp;&quot;&#39;&gt;");
    }
}
