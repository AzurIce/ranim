export default `
  :host {
    display: block;
    min-width: 0;
    --_bg: #faf8f4;
    --_fg: #30332f;
    --_muted: #686e64;
    --_line: #e1e2d9;
    --_surface: #f5f4ee;
    --_surface-alt: #f0f1eb;
    --_accent: #3d6655;
    --pele-bg: var(--bg, var(--_bg));
    --pele-fg: var(--fg, var(--_fg));
    --pele-muted: var(--muted, var(--_muted));
    --pele-line: var(--muted, var(--_muted));
    --pele-border: var(--line, var(--_line));
    --pele-surface: var(--surface, var(--_surface));
    --pele-surface-alt: var(--code, var(--_surface-alt));
    --pele-accent: var(--accent, var(--_accent));
    --pele-font: var(--font-ui, sans-serif);
    --pele-font-mono: var(--font-code, monospace);
  }
  :host([data-pele-scheme="dark"]) {
    --_bg: #1c211e;
    --_fg: #dedfd5;
    --_muted: #a3aba0;
    --_line: #363e36;
    --_surface: #222923;
    --_surface-alt: #252d27;
    --_accent: #a5c7ad;
  }
  :host([notist-theme="light"]), :host([notist-theme="dark"]) {
    --pele-bg: var(--_bg);
    --pele-fg: var(--_fg);
    --pele-muted: var(--_muted);
    --pele-line: var(--_muted);
    --pele-border: var(--_line);
    --pele-surface: var(--_surface);
    --pele-surface-alt: var(--_surface-alt);
    --pele-accent: var(--_accent);
  }
  .diagram { min-width: 0; background: var(--pele-bg); border-radius: var(--pele-radius, 4px); }
  .diagram > svg { display: block; max-width: 100%; height: auto; margin-inline: auto; }
  .load-error { margin: 0; color: var(--pele-error, #b44241); white-space: pre-wrap; font: inherit; }
`;
