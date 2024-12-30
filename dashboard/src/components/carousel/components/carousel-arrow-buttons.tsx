import type { Theme } from '@mui/material/styles';

import { ArrowButton } from './arrow-button';

import type { CarouselArrowButtonsProps } from '../types';

// ----------------------------------------------------------------------

export function CarouselArrowFloatButtons({
  sx,
  options,
  slotProps,
  onClickPrev,
  onClickNext,
  disablePrev,
  disableNext,
}: CarouselArrowButtonsProps) {
  const baseStyles = (theme: Theme) => ({
    zIndex: 9,
    top: '50%',
    borderRadius: 1.5,
    position: 'absolute',
    color: 'common.white',
    bgcolor: 'text.primary',
    '&:hover': { opacity: 0.8 },
    ...theme.applyStyles('dark', {
      color: 'grey.800',
    }),
  });

  return (
    <>
      <ArrowButton
        variant="prev"
        options={options}
        disabled={disablePrev}
        onClick={onClickPrev}
        svgIcon={slotProps?.prevBtn?.svgIcon}
        svgSize={slotProps?.prevBtn?.svgSize}
        sx={[
          (theme) => ({
            ...baseStyles(theme),
            left: 0,
            transform: 'translate(-50%, -50%)',
          }),
          ...(Array.isArray(sx) ? sx : [sx]),
          ...(Array.isArray(slotProps?.prevBtn?.sx)
            ? (slotProps?.prevBtn?.sx ?? [])
            : [slotProps?.prevBtn?.sx]),
        ]}
      />

      <ArrowButton
        variant="next"
        options={options}
        disabled={disableNext}
        onClick={onClickNext}
        svgIcon={slotProps?.nextBtn?.svgIcon}
        svgSize={slotProps?.nextBtn?.svgSize}
        sx={[
          (theme) => ({
            ...baseStyles(theme),
            right: 0,
            transform: 'translate(50%, -50%)',
          }),
          ...(Array.isArray(sx) ? sx : [sx]),
          ...(Array.isArray(slotProps?.nextBtn?.sx)
            ? (slotProps?.nextBtn?.sx ?? [])
            : [slotProps?.nextBtn?.sx]),
        ]}
      />
    </>
  );
}
