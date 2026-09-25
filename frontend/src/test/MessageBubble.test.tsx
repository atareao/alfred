import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { MessageBubble } from '../components/MessageBubble';
import type { Message } from '../types';

describe('MessageBubble', () => {
  it('renders user message right-aligned', () => {
    const msg: Message = {
      id: '1', role: 'user',
      content: 'Hola', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.justifyContent).toBe('flex-end');
  });

  it('renders assistant message left-aligned', () => {
    const msg: Message = {
      id: '2', role: 'assistant',
      content: 'Hola, soy Alfred', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.justifyContent).toBe('flex-start');
  });

  it('renders system message centered', () => {
    const msg: Message = {
      id: '3', role: 'system',
      content: 'System message', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.justifyContent).toBe('center');
  });

  it('renders tool role with monospace style', () => {
    const msg: Message = {
      id: '4', role: 'tool',
      content: 'Tool output', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    const bubble = container.firstChild as HTMLElement;
    expect(bubble.style.fontFamily).toBe('monospace');
  });

  // ════════════════════════════════════════════════════════════════
  // RED phase tests — expected to FAIL until react-markdown is added
  // ════════════════════════════════════════════════════════════════

  it('renders assistant markdown bold without literal asterisks', () => {
    const msg: Message = {
      id: 'markdown-bold', role: 'assistant',
      content: 'Hello **world**', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    // BUG: Currently renders plain text, so "**world**" literal appears in output.
    // FIX: After react-markdown, bold text should not contain literal asterisks.
    expect(container.textContent).not.toContain('**world**');
  });

  it('renders assistant markdown code block as formatted code', () => {
    const msg: Message = {
      id: 'markdown-code', role: 'assistant',
      content: '```rust\nfn main() {}\n```', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    // BUG: Currently renders backticks literally. Fix should render formatted code block.
    expect(container.textContent).not.toContain('```rust');
  });

  it('renders user message as plain text (no markdown processing)', () => {
    const msg: Message = {
      id: 'user-plain', role: 'user',
      content: 'Hello **world**', created_at: '2024-01-01T00:00:00Z',
    };
    const { container } = render(<MessageBubble message={msg} />);
    // User messages should remain plain text even if they contain markdown characters.
    expect(container.textContent).toContain('**world**');
  });
});