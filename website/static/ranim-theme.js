/** Shared host-aware tokens for the viewport, controls, and nested timeline. */
export const previewTheme = `
  :host {
    color-scheme: inherit;
    --accent: var(--ranim-accent, var(--sl-color-primary-600, var(--link-color, #0284c7)));
    --surface: var(--ranim-controls-background, var(--bgColor-default, var(--main-background-color, light-dark(#fff, #1e1e1e))));
    --text: var(--ranim-text, var(--fgColor-default, var(--main-color, light-dark(#1f2937, #e5e7eb))));
    --muted: var(--ranim-muted, var(--fgColor-muted, color-mix(in srgb, var(--text) 65%, var(--surface))));
    --border: var(--ranim-border, var(--borderColor-muted, color-mix(in srgb, var(--text) 16%, var(--surface))));
    --subtle: color-mix(in srgb, var(--text) 3%, var(--surface));
    --hover: color-mix(in srgb, var(--text) 6%, var(--surface));
    --line: color-mix(in srgb, var(--text) 7%, var(--surface));
    --accent-soft: color-mix(in srgb, var(--accent) 9%, var(--surface));
    --mono: var(--font-family-mono, var(--font-family-code, ui-monospace, SFMono-Regular, Consolas, monospace));
    --radius: var(--ranim-radius, 8px);
    color: var(--text);
    font-family: inherit;
  }
`;
