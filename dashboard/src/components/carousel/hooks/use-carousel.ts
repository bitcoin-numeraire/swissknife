import type { EmblaPluginType } from 'embla-carousel';

import { useMemo } from 'react';
import useEmblaCarousel from 'embla-carousel-react';

import { useTheme } from '@mui/material/styles';

import { useThumbs } from './use-thumbs';
import { useCarouselDots } from './use-carousel-dots';
import { useParallax } from './use-carousel-parallax';
import { useCarouselArrows } from './use-carousel-arrows';
import { useCarouselProgress } from './use-carousel-progress';

import type { CarouselOptions, UseCarouselReturn } from '../types';

// ----------------------------------------------------------------------

export const useCarousel = (
  options?: CarouselOptions,
  plugins?: EmblaPluginType[]
): UseCarouselReturn => {
  const theme = useTheme();

  const [mainRef, mainApi] = useEmblaCarousel({ ...options, direction: theme.direction }, plugins);

  const { disablePrev, disableNext, onClickPrev, onClickNext } = useCarouselArrows(mainApi);

  const pluginNames = plugins?.map((plugin) => plugin.name);

  const _dots = useCarouselDots(mainApi);

  const _progress = useCarouselProgress(mainApi);

  const _thumbs = useThumbs(mainApi, options?.thumbs);

  useParallax(mainApi, options?.parallax);

  const controls = useMemo(() => ({ onClickPrev, onClickNext }), [onClickNext, onClickPrev]);

  const mergedOptions = { ...options, ...mainApi?.internalEngine().options };

  return {
    options: mergedOptions,
    pluginNames,
    mainRef,
    mainApi,
    // arrows
    arrows: {
      disablePrev,
      disableNext,
      onClickPrev: controls.onClickPrev,
      onClickNext: controls.onClickNext,
    },
    // dots
    dots: _dots,
    // thumbs
    thumbs: _thumbs,
    // progress
    progress: _progress,
  };
};
