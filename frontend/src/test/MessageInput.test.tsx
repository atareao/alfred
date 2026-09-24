import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MessageInput } from '../components/MessageInput';

describe('MessageInput', () => {
  it('disables button when component is disabled', () => {
    render(<MessageInput onSend={vi.fn()} disabled={true} />);
    const button = screen.getByRole('button');
    expect(button).toBeDisabled();
  });

  it('shows loading state when sending (disabled + spinner)', () => {
    render(<MessageInput onSend={vi.fn()} disabled={true} />);
    // FALLA: el stub no muestra spinner en el botón cuando está disabled
    const button = screen.getByRole('button');
    expect(button.querySelector('.anticon-loading')).toBeInTheDocument();
  });
});