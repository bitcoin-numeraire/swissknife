import type { UseCarouselReturn } from 'src/components/carousel';

import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';
import {
  Carousel,
  useCarousel,
  CarouselDotButtons,
  CarouselArrowBasicButtons,
} from 'src/components/carousel';

// ----------------------------------------------------------------------

export type SlideData = {
  id: string;
  title: string;
  content: string;
  icon?: string;
};

type Props = {
  data: SlideData[];
  onComplete: () => void;
};

export function WelcomeCarousel({ data, onComplete }: Props) {
  const carousel = useCarousel();

  return (
    <Box
      sx={{
        width: '100%',
      }}
    >
      <Carousel carousel={carousel}>
        {data.map((item, index) => (
          <CarouselItem
            key={item.id}
            item={item}
            isLast={index === data.length - 1}
            onComplete={onComplete}
            carousel={carousel}
          />
        ))}
      </Carousel>

      <CarouselDotButtons
        variant="rounded"
        scrollSnaps={carousel.dots.scrollSnaps}
        selectedIndex={carousel.dots.selectedIndex}
        onClickDot={carousel.dots.onClickDot}
        sx={{ justifyContent: 'center', mt: 4 }}
      />
    </Box>
  );
}

// ----------------------------------------------------------------------

type CarouselItemProps = {
  item: Props['data'][number];
  isLast: boolean;
  onComplete: () => void;
  carousel: UseCarouselReturn;
};

function CarouselItem({ item, isLast, onComplete, carousel }: CarouselItemProps) {
  const { t } = useTranslate('welcome');

  return (
    <Box
      sx={{
        maxWidth: { md: '50%' },
        m: 'auto',
        p: { xs: 2, md: 0 },
      }}
    >
      {item.icon && (
        <Box
          sx={{
            width: { xs: '48px', md: '64px' },
            height: { xs: '48px', md: '64px' },
            m: 'auto',
            mb: 2,
          }}
        >
          <Iconify icon={item.icon} sx={{ width: '100%', height: '100%' }} />
        </Box>
      )}

      <Typography variant="h2" sx={{ mb: 1, fontWeight: 600 }}>
        {t(item.title)}
      </Typography>

      <Typography variant="h5" sx={{ color: 'text.secondary' }}>
        {t(item.content)}
      </Typography>

      {isLast ? (
        <Button variant="contained" onClick={onComplete} sx={{ mt: 4 }}>
          {t('get_started')}
        </Button>
      ) : (
        <CarouselArrowBasicButtons
          sx={(theme) => ({
            borderRadius: 1.5,
            bgcolor: 'text.primary',
            '&:hover': { opacity: 0.8 },
            ...theme.applyStyles('dark', {
              color: 'grey.800',
            }),
            mt: 4,
          })}
          {...carousel.arrows}
          options={carousel.options}
          slotProps={{ prevBtn: { sx: { display: 'none' } } }}
        />
      )}
    </Box>
  );
}
