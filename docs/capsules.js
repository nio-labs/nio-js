document.addEventListener('DOMContentLoaded', () => {
  const picker = document.querySelector('[data-format-picker]');
  if (picker) {
    const tabs = [...picker.querySelectorAll('[role="tab"]')];
    const panels = [...picker.querySelectorAll('[data-panel]')];
    const select = (tab, focus = false) => {
      tabs.forEach(item => {
        const active = item === tab;
        item.setAttribute('aria-selected', String(active));
        item.tabIndex = active ? 0 : -1;
      });
      panels.forEach(panel => { panel.hidden = panel.dataset.panel !== tab.dataset.format; });
      if (focus) tab.focus();
    };
    panels.forEach(panel => {
      panel.setAttribute('role', 'tabpanel');
      panel.setAttribute('aria-labelledby', `tab-${panel.dataset.panel}`);
      panel.tabIndex = 0;
    });
    picker.querySelector('[role="tablist"]').hidden = false;
    tabs.forEach((tab, index) => {
      tab.addEventListener('click', () => select(tab));
      tab.addEventListener('keydown', event => {
        const target = { ArrowRight: (index + 1) % tabs.length, ArrowLeft: (index + tabs.length - 1) % tabs.length, Home: 0, End: tabs.length - 1 }[event.key];
        if (target !== undefined) { event.preventDefault(); select(tabs[target], true); }
      });
    });
    select(tabs[0]);
  }

  if (navigator.clipboard?.writeText) {
    document.querySelectorAll('main pre').forEach((pre, index) => {
      const code = pre.querySelector('code');
      if (!code) return;
      const wrapper = document.createElement('div');
      wrapper.className = 'capsule-code-block';
      pre.before(wrapper);
      wrapper.append(pre);
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'capsule-copy';
      button.textContent = 'Copy';
      button.setAttribute('aria-label', `Copy code example ${index + 1}`);
      button.addEventListener('click', async () => {
        try { await navigator.clipboard.writeText(code.textContent); button.textContent = 'Copied'; }
        catch { button.textContent = 'Select to copy'; }
        setTimeout(() => { button.textContent = 'Copy'; }, 2000);
      });
      wrapper.append(button);
    });
  }
});
