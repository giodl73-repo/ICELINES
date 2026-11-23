import type {NumericalEvidence} from './rinkcraft-numbers.js';
import type {HistoryCard} from './rinkcraft-history.js';
// Presentation only. Values, missingness, rates and changes come from Rust.
import {renderAttributeSheet} from './rinkcraft-attributes.js';
import type {AttributeSeason} from './rinkcraft-attributes.js';
import {renderCharacterCard} from './rinkcraft-character.js';
import type {CharacterSheet} from './rinkcraft-character.js';
export interface ProfileMetric {
  key: string; label: string; unit: string; value: number | null; status: string;
  aggregation: string; observed_seasons: number; missing_seasons: number[];
  report_source: string | null;
  ratio: { numerator_key: string; denominator_key: string; numerator: number; denominator: number; scale: number } | null;
}
export interface ProfileSeason { season: number; games: number; position: string; teams: string[]; traded: boolean; metrics: ProfileMetric[]; }
export interface RinkcraftProfile {
  stat_attribute_seasons?:import('./rinkcraft-stat-scores.js').StatProfile[];
  record_milestones?:import('./rinkcraft-records.js').RecordMilestones;
  scope_variants?:{current:RinkcraftProfile;career?:RinkcraftProfile};
  profile_window?:import('./rinkcraft-scope.js').ProfileTimeWindow;
  experimental_patterns?:import('./rinkcraft-scope.js').ExperimentalPatterns;
  achievements?:import('./rinkcraft-achievements.js').AchievementProfile;
  deployment_evidence?:import('./rinkcraft-deployment.js').DeploymentEvidence;
  evidence_guide?:import('./rinkcraft-evidence-guide.js').EvidenceGuideTopic[];
  numerical_history?:HistoryCard;
  numerical_snapshot?:import('./rinkcraft-numbers.js').SnapshotSelection;
  documentary_catalog_available?:boolean;
  numerical_comparison_choices?:import('./rinkcraft-numbers.js').ComparisonChoice[];
  numerical_evidence?:NumericalEvidence;
  character_sheet?:CharacterSheet;
  attribute_seasons?:AttributeSeason[];
  schema: string; player_name: string; request: { player_id?:number; season_type: string }; scope: string;
  included_seasons: number[]; current_season_available: boolean; latest_available: ProfileSeason | null;
  summary: ProfileMetric[]; timeline: ProfileSeason[];
  changes: { from: number; to: number; previous_games: number; latest_games: number; metrics: { key: string; difference: number | null }[] }[];
  job_families: { name: string; catalog_keys: string[]; interpretation: string }[];
  unmeasured_dimensions: string[]; limitations: string[];
}
export function profileValue(metric: Pick<ProfileMetric, 'unit' | 'value'>): string {
  const v = metric.value;
  if (v === null || !Number.isFinite(v)) return '—';
  if (metric.unit === 'Pct') return `${(v * 100).toFixed(2)}%`;
  if (metric.unit === 'Seconds') return `${v.toFixed(1)} s`;
  return v.toLocaleString('en-US', { maximumFractionDigits: metric.unit === 'Count' ? 0 : 3 });
}
const seasonLabel = (s: number): string => `${Math.floor(s / 10000)}–${String(s % 100).padStart(2, '0')}`;
function node<K extends keyof HTMLElementTagNameMap>(tag: K, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag); if (text !== undefined) e.textContent = text; return e;
}
export function renderRinkcraftProfile(profile: RinkcraftProfile): HTMLElement {
  const section = node('section'); section.id = 'rinkcraft-profile'; section.className = 'rinkcraft-profile';
  const targetSeason=profile.numerical_evidence?.season??profile.latest_available?.season;
  const cardAssessment=profile.attribute_seasons?.find(s=>s.season===targetSeason);
  section.append(renderCharacterCard(profile.player_name,profile.character_sheet,cardAssessment,profile.timeline.find(s=>s.season===targetSeason)??profile.latest_available??undefined,profile.numerical_evidence,profile.numerical_history,profile.numerical_snapshot,profile.numerical_comparison_choices,profile.documentary_catalog_available,profile.evidence_guide,profile.deployment_evidence,profile.request,profile.achievements,profile.profile_window,profile.experimental_patterns,undefined,profile.record_milestones,profile.stat_attribute_seasons));
  const history=node('details');history.className='rk-history';history.append(node('summary','Trait evolution, time travel & statistical evidence'));section.append(history);
  const content=node('div');history.append(content);
  content.append(node('h3', 'Rinkcraft profile'), node('p', `${profile.request.season_type} · ${profile.included_seasons.length} loaded seasons · Latest: ${profile.latest_available ? seasonLabel(profile.latest_available.season) : 'unavailable'}`));
  content.append(node('p', 'Statistical evidence is separate from the documentary identity above. Resident windows are not a verified complete career.'));
  const controls = node('div'); controls.className = 'actions';
  const scopeLabel = node('label', 'View '); const scope = node('select'); scope.setAttribute('aria-label', 'Rinkcraft season view');
  const all = node('option', 'Loaded window summary'); all.value = 'summary'; scope.append(all);
  for (const s of [...profile.timeline].reverse()) { const o = node('option', `${seasonLabel(s.season)} · ${s.games} GP`); o.value = String(s.season); scope.append(o); }
  scope.value = profile.latest_available ? String(profile.latest_available.season) : 'summary'; scopeLabel.append(scope);
  const searchLabel = node('label', 'Find metric '); const search = node('input'); search.type = 'search'; search.placeholder = 'Goals, assists, save-pct…'; searchLabel.append(search);
  const missingLabel = node('label'); const missing = node('input'); missing.type = 'checkbox'; missingLabel.append(missing, node('span', ' Show unavailable metrics'));
  controls.append(scopeLabel, searchLabel, missingLabel); content.append(controls);
  const context = node('p'); context.setAttribute('aria-live', 'polite');
  const families = node('div'); families.className = 'rinkcraft-families';
  for (const f of profile.job_families) {
    const card = node('details'); card.append(node('summary', f.name), node('p', f.interpretation), node('p', f.catalog_keys.join(' · ') || 'No measurements in this source'));
    families.append(card);
  }
  const attributeHolder=node('div');content.append(attributeHolder,families,context);
  const scroll = node('div'); scroll.className = 'table-scroll'; const table = node('table');
  const head = node('thead'); const hr = node('tr'); for (const t of ['Metric / evidence', 'Value', 'State']) hr.append(node('th', t)); head.append(hr);
  const body = node('tbody'); table.append(head, body); scroll.append(table); content.append(scroll);
  const render = (): void => {
    const selected = profile.timeline.find(s => String(s.season) === scope.value);
    const assessments=profile.attribute_seasons??[];
    attributeHolder.replaceChildren(renderAttributeSheet(assessments.find(s=>s.season===selected?.season)??assessments.at(-1),assessments));
    const metrics = selected?.metrics ?? profile.summary;
    const rows = metrics.filter(m => (missing.checked || m.value !== null) && `${m.label} ${m.key}`.toLowerCase().includes(search.value.toLowerCase()));
    context.textContent = selected ? `${selected.games} GP · ${selected.position} · ${selected.teams.join(', ')} · Traded: ${selected.traded} · ${rows.length} matching metrics` : `Loaded window summary · ${rows.length} matching metrics. Rates use supported denominators; incomplete coverage stays unavailable.`;
    body.replaceChildren();
    for (const m of rows) {
      const row = node('tr'); const label = node('td'); const evidence = node('details'); evidence.append(node('summary', `${m.label} · ${m.key}`));
      evidence.append(node('p', `${m.unit} · ${m.aggregation} · Source report: ${m.report_source ?? 'not specified'} · ${m.observed_seasons} observed seasons · Missing: ${m.missing_seasons.map(seasonLabel).join(', ') || 'none'}`));
      if (m.ratio) { const r = m.ratio; evidence.append(node('p', `${r.numerator_key} (${r.numerator}) / ${r.denominator_key} (${r.denominator}) × ${r.scale}`)); }
      label.append(evidence); row.append(label, node('td', profileValue(m)), node('td', m.status)); body.append(row);
    }
  };
  scope.onchange = render; missing.onchange = render; search.oninput = render; render();
  const changes = node('details'); changes.append(node('summary', 'Season changes'), node('p', 'Arithmetic differences only. Counts depend on games and deployment; differences do not establish role changes.'));
  for (const c of profile.changes) {
    const window = node('details'); window.append(node('summary', `${seasonLabel(c.from)} → ${seasonLabel(c.to)} · ${c.previous_games} → ${c.latest_games} GP`));
    const list = node('ul');
    for (const m of c.metrics.filter(m => m.difference !== null)) {
      const metric = profile.summary.find(s => s.key === m.key);
      list.append(node('li', `${metric?.label ?? m.key}: ${profileValue({unit: metric?.unit ?? '', value: m.difference})}${metric?.unit === 'Pct' ? ' (percentage points)' : ''}`));
    }
    window.append(list); changes.append(window);
  }
  const limits = node('details'); limits.append(node('summary', 'Methods, unmeasured dimensions and limits'));
  for (const l of profile.limitations) limits.append(node('p', l));
  limits.append(node('p', `Unmeasured: ${profile.unmeasured_dimensions.join(', ')}`));
  content.append(changes, limits); return section;
}
