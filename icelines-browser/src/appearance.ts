export const THEMES = ['night-game', 'fresh-ice', 'hockey-club'] as const;
export type Theme = typeof THEMES[number];
export const THEME_KEY = 'icelines.appearance.v1';
export function parseTheme(value: unknown): Theme {
  return THEMES.find(theme => theme === value) ?? 'night-game';
}

export function initializeAppearance(): void {
  const picker = document.getElementById('appearance') as HTMLSelectElement;
  const notice = document.getElementById('appearance-status')!;
  function apply(theme: Theme): void {
    document.documentElement.dataset.theme = theme;
    picker.value = theme;
    const color = getComputedStyle(document.documentElement).getPropertyValue('--bg').trim();
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', color);
  }
  try { apply(parseTheme(localStorage.getItem(THEME_KEY))); }
  catch { apply('night-game'); }
  picker.addEventListener('change', () => {
    const theme = parseTheme(picker.value);
    apply(theme);
    try {
      localStorage.setItem(THEME_KEY, theme);
      notice.textContent = 'Appearance saved on this device.';
    } catch {
      notice.textContent = 'Appearance changed for this tab. Your browser could not save the preference.';
    }
  });
  window.addEventListener('storage', event => {
    if (event.key === THEME_KEY || event.key === null) apply(parseTheme(event.newValue));
  });
}
