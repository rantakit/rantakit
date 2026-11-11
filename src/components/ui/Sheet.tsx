'use client';
import {ModalOverlay, type ModalOverlayProps, Modal as RACModal} from 'react-aria-components/Modal';
import {tv} from '@/lib/react-aria-utils';

const overlayStyles = tv({
  base: 'absolute top-0 left-0 w-full h-(--page-height) isolate z-20 bg-backdrop',
  variants: {
    isEntering: {
      true: 'animate-in fade-in duration-200 ease-out'
    },
    isExiting: {
      true: 'animate-out fade-out duration-200 ease-in'
    }
  }
});

const sheetStyles = tv({
  base: 'font-sans w-full max-w-[min(100vw,480px)] max-h-[calc(var(--visual-viewport-height)*.9)] overflow-y-auto rounded-t-[20px] md:rounded-[20px] bg-surface text-left text-text bg-clip-padding border border-border',
  variants: {
    isEntering: {
      true: 'animate-in slide-in-from-bottom-8 md:slide-in-from-bottom-0 md:zoom-in-105 ease-out duration-200'
    },
    isExiting: {
      true: 'animate-out slide-out-to-bottom-8 md:slide-out-to-bottom-0 md:zoom-out-95 ease-in duration-200'
    }
  }
});

/**
 * A dialog that rises from the bottom of a phone screen, within reach of a
 * thumb; centered like `Modal` on a wider screen. Put a `Dialog` inside.
 */
export function Sheet(props: ModalOverlayProps) {
  return (
    <ModalOverlay {...props} className={overlayStyles}>
      <div className="sticky top-0 left-0 w-full h-(--visual-viewport-height) flex items-end md:items-center justify-center box-border">
        <RACModal {...props} className={sheetStyles} />
      </div>
    </ModalOverlay>
  );
}
