'use client';
import {Link as AriaLink, type LinkProps as AriaLinkProps} from 'react-aria-components/Link';
import {composeRenderProps} from 'react-aria-components/composeRenderProps';
import {focusRing, tv} from '@/lib/react-aria-utils';

const styles = tv({
  extend: focusRing,
  base: 'inline-flex items-center gap-1 rounded-xs font-semibold text-brand-strong no-underline underline-offset-4 hover:underline cursor-pointer disabled:text-text-disabled disabled:no-underline disabled:cursor-default forced-colors:disabled:text-[GrayText] [-webkit-tap-highlight-color:transparent]'
});

/** A text link: brand-colored text without a box, underlined on hover. Icons inside are 16 px. */
export function Link(props: AriaLinkProps) {
  return (
    <AriaLink
      {...props}
      className={composeRenderProps(props.className, (className, renderProps) =>
        styles({...renderProps, className})
      )}
    />
  );
}
