import './ranim-viewport.js';
import './ranim-controls.js';

for (const preview of document.querySelectorAll('.ranim-preview')) {
  preview.querySelector('ranim-controls').target = preview.querySelector('ranim-viewport');
}

for (const heading of document.querySelectorAll('article :is(h1,h2,h3,h4,h5,h6)[id]')) {
  const anchor = document.createElement('a');
  anchor.className = 'heading-anchor';
  anchor.href = `#${encodeURIComponent(heading.id)}`;
  anchor.setAttribute('aria-label', `Anchor link for: ${heading.textContent}`);
  anchor.textContent = '⁋';
  heading.append(anchor);
}
