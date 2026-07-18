export function useMarkdown() {
  function escapeHtml(value: string): string {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function sanitizeMarkdownLinkUrl(url: string): string | null {
    const trimmed = url.trim();
    if (!trimmed) return null;

    let candidate = trimmed;
    if (!/^[\w-]+:/i.test(candidate)) {
      candidate = `https://${candidate}`;
    }

    try {
      const parsed = new URL(candidate);
      const protocol = parsed.protocol.toLowerCase();
      if (protocol === 'http:' || protocol === 'https:' || protocol === 'mailto:') {
        return candidate;
      }
    } catch {
      return null;
    }

    return null;
  }

  function formatMarkdownInline(raw: string): string {
    const linkPattern = /\[([^\]]+)\]\(([^)]+)\)/g;
    let result = '';
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = linkPattern.exec(raw)) !== null) {
      result += escapeHtml(raw.slice(lastIndex, match.index));
      const label = match[1] ?? '';
      const href = sanitizeMarkdownLinkUrl(match[2] ?? '');
      if (href) {
        result += `<a class="md-link" href="${escapeHtml(href)}" target="_blank" rel="noopener noreferrer">${escapeHtml(label)}</a>`;
      } else {
        result += escapeHtml(match[0]);
      }
      lastIndex = match.index + match[0].length;
    }

    result += escapeHtml(raw.slice(lastIndex));
    return result.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  }

  function renderSimpleMarkdown(markdown: string): string {
    return markdown
      .trim()
      .split('\n')
      .map((line) => {
        const trimmed = line.trim();
        if (!trimmed) return '<div class="md-spacer"></div>';

        const blockquote = trimmed.match(/^>\s?(.*)$/);
        if (blockquote) {
          return `<blockquote><p>${formatMarkdownInline(blockquote[1] ?? '')}</p></blockquote>`;
        }

        const heading = trimmed.match(/^(#{1,6})\s+(.+)$/);
        if (heading) {
          const level = Math.min(6, Math.max(1, heading[1].length));
          return `<h${level}>${formatMarkdownInline(heading[2])}</h${level}>`;
        }

        if (trimmed.startsWith('- ')) {
          return `<li>${formatMarkdownInline(trimmed.slice(2))}</li>`;
        }

        return `<p>${formatMarkdownInline(trimmed)}</p>`;
      })
      .join('')
      .replace(/(<li>.*?<\/li>)+/g, (match) => `<ul>${match}</ul>`)
      .replace(/<\/blockquote><blockquote>/g, '');
  }

  return { renderSimpleMarkdown };
}
