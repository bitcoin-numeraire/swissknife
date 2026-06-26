import { varAlpha } from 'minimal-shared/utils';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';

import { Iconify } from 'src/components/iconify';
import { Carousel, useCarousel, CarouselDotButtons } from 'src/components/carousel';

// ----------------------------------------------------------------------

export type SlideData = {
  id: string;
  title: string;
  accent: string;
  content: string;
  icon: string;
  points: string[];
};

type Props = {
  data: SlideData[];
  onComplete: () => void;
};

export function WelcomeCarousel({ data, onComplete }: Props) {
  const { t } = useTranslate('welcome');
  const carousel = useCarousel({ loop: false });

  const selectedIndex = carousel.dots.selectedIndex;
  const selectedSlide = data[selectedIndex] ?? data[0];
  const isLast = selectedIndex === data.length - 1;

  return (
    <Box
      sx={(theme) => ({
        width: 1,
        minHeight: '100dvh',
        display: 'grid',
        overflowX: 'hidden',
        gridTemplateColumns: { xs: '1fr', md: 'minmax(420px, 0.95fr) minmax(0, 1fr)' },
        backgroundImage: [
          `linear-gradient(90deg, ${varAlpha(theme.vars.palette.grey['500Channel'], 0.05)} 1px, transparent 1px)`,
          `linear-gradient(0deg, ${varAlpha(theme.vars.palette.grey['500Channel'], 0.05)} 1px, transparent 1px)`,
        ].join(','),
        backgroundSize: '48px 48px',
      })}
    >
      <Stack
        spacing={4}
        sx={(theme) => ({
          p: { xs: 3, md: 6 },
          width: 1,
          minWidth: 0,
          maxWidth: '100vw',
          alignItems: { xs: 'flex-start', md: 'stretch' },
          minHeight: { md: '100dvh' },
          justifyContent: 'center',
          borderRight: {
            md: `1px solid ${varAlpha(theme.vars.palette.grey['500Channel'], 0.14)}`,
          },
          bgcolor: 'grey.900',
          color: 'common.white',
        })}
      >
        <Stack spacing={1.5} sx={{ maxWidth: 460 }}>
          <Typography
            variant="overline"
            sx={{ color: 'primary.light', letterSpacing: 0, fontWeight: 700 }}
          >
            {t('preview.eyebrow')}
          </Typography>
          <Typography
            variant="h2"
            sx={{
              maxWidth: { xs: 320, md: 460 },
              fontSize: { xs: 32, md: 52 },
              overflowWrap: 'break-word',
            }}
          >
            {t('preview.title')}
          </Typography>
          <Typography
            sx={(theme) => ({
              maxWidth: { xs: 320, md: 420 },
              color: varAlpha(theme.vars.palette.common.whiteChannel, 0.7),
            })}
          >
            {t('preview.content')}
          </Typography>
        </Stack>

        <WalletPreview item={selectedSlide} step={selectedIndex + 1} total={data.length} />
      </Stack>

      <Stack
        spacing={4}
        sx={{
          p: { xs: 3, sm: 5, md: 8 },
          width: 1,
          minWidth: 0,
          maxWidth: '100vw',
          minHeight: { md: '100dvh' },
          justifyContent: 'center',
        }}
      >
        <Stack spacing={1}>
          <Typography variant="overline" sx={{ color: 'text.secondary', letterSpacing: 0 }}>
            {t('step', { current: selectedIndex + 1, total: data.length })}
          </Typography>

          <Carousel carousel={carousel}>
            {data.map((item) => (
              <CarouselItem key={item.id} item={item} />
            ))}
          </Carousel>
        </Stack>

        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={2}
          sx={{ alignItems: { xs: 'stretch', sm: 'center' }, justifyContent: 'space-between' }}
        >
          <CarouselDotButtons
            variant="rounded"
            scrollSnaps={carousel.dots.scrollSnaps}
            selectedIndex={selectedIndex}
            onClickDot={carousel.dots.onClickDot}
            sx={{ justifyContent: { xs: 'center', sm: 'flex-start' } }}
          />

          <Stack
            direction="row"
            spacing={1.5}
            sx={{ justifyContent: { xs: 'center', sm: 'flex-end' } }}
          >
            <Button
              variant="outlined"
              color="inherit"
              disabled={carousel.arrows.disablePrev}
              onClick={carousel.arrows.onClickPrev}
              startIcon={<Iconify icon="solar:arrow-left-linear" />}
            >
              {t('back')}
            </Button>

            <Button
              variant="contained"
              onClick={isLast ? onComplete : carousel.arrows.onClickNext}
              endIcon={
                <Iconify icon={isLast ? 'solar:login-3-bold' : 'solar:arrow-right-linear'} />
              }
            >
              {isLast ? t('get_started') : t('continue')}
            </Button>
          </Stack>
        </Stack>
      </Stack>
    </Box>
  );
}

// ----------------------------------------------------------------------

type CarouselItemProps = {
  item: SlideData;
};

function CarouselItem({ item }: CarouselItemProps) {
  const { t } = useTranslate('welcome');

  return (
    <Stack spacing={3} sx={{ minHeight: 360, justifyContent: 'center' }}>
      <Stack spacing={2}>
        <Box
          sx={{
            width: 56,
            height: 56,
            borderRadius: 1,
            display: 'grid',
            placeItems: 'center',
            color: item.accent,
            bgcolor: 'background.neutral',
          }}
        >
          <Iconify icon={item.icon} width={30} />
        </Box>

        <Typography variant="h2" sx={{ maxWidth: 620 }}>
          {t(item.title)}
        </Typography>

        <Typography variant="h6" sx={{ color: 'text.secondary', maxWidth: 620, fontWeight: 400 }}>
          {t(item.content)}
        </Typography>
      </Stack>

      <Box
        sx={{
          gap: 1.5,
          display: 'grid',
          gridTemplateColumns: { xs: '1fr', sm: 'repeat(3, minmax(0, 1fr))' },
        }}
      >
        {item.points.map((point) => (
          <Stack
            key={point}
            spacing={1}
            sx={(theme) => ({
              p: 2,
              minHeight: 118,
              borderRadius: 1,
              border: `1px solid ${varAlpha(theme.vars.palette.grey['500Channel'], 0.16)}`,
              bgcolor: 'background.paper',
            })}
          >
            <Iconify
              icon="solar:check-circle-bold-duotone"
              width={22}
              sx={{ color: item.accent }}
            />
            <Typography variant="body2" sx={{ color: 'text.secondary' }}>
              {t(point)}
            </Typography>
          </Stack>
        ))}
      </Box>
    </Stack>
  );
}

type WalletPreviewProps = {
  item: SlideData;
  step: number;
  total: number;
};

function WalletPreview({ item, step, total }: WalletPreviewProps) {
  const { t } = useTranslate('welcome');

  return (
    <Box
      sx={(theme) => ({
        p: 2,
        width: { xs: 320, sm: 1 },
        maxWidth: { xs: 320, sm: 460 },
        borderRadius: 1,
        border: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.12)}`,
        bgcolor: varAlpha(theme.vars.palette.common.whiteChannel, 0.06),
        boxShadow: `0 32px 96px ${varAlpha(theme.vars.palette.common.blackChannel, 0.34)}`,
        backdropFilter: 'blur(10px)',
      })}
    >
      <Stack spacing={2.5}>
        <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
          <Box
            component="img"
            alt=""
            src={`${CONFIG.assetsDir}/assets/icons/bitcoin/ic-bitcoin-lightning.svg`}
            sx={{ width: 42, height: 42 }}
          />

          <Box sx={{ minWidth: 0, flex: '1 1 auto' }}>
            <Typography variant="subtitle1">{t('preview.wallet_title')}</Typography>
            <Typography
              variant="caption"
              sx={(theme) => ({ color: varAlpha(theme.vars.palette.common.whiteChannel, 0.58) })}
            >
              {t('preview.wallet_caption')}
            </Typography>
          </Box>

          <Box
            sx={(theme) => ({
              px: 1,
              py: 0.5,
              borderRadius: 0.75,
              color: item.accent,
              typography: 'caption',
              bgcolor: varAlpha(theme.vars.palette.common.whiteChannel, 0.08),
            })}
          >
            {step}/{total}
          </Box>
        </Stack>

        <Box
          sx={(theme) => ({
            p: 2,
            borderRadius: 1,
            border: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.1)}`,
            bgcolor: varAlpha(theme.vars.palette.common.blackChannel, 0.24),
          })}
        >
          <Stack spacing={0.5}>
            <Typography
              variant="caption"
              sx={(theme) => ({ color: varAlpha(theme.vars.palette.common.whiteChannel, 0.52) })}
            >
              {t('preview.available')}
            </Typography>
            <Typography variant="h3">0 sats</Typography>
            <Typography
              variant="body2"
              sx={(theme) => ({ color: varAlpha(theme.vars.palette.common.whiteChannel, 0.62) })}
            >
              {t('preview.empty_state')}
            </Typography>
          </Stack>
        </Box>

        <Box sx={{ gap: 1, display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' }}>
          {[
            ['solar:bolt-bold-duotone', 'preview.lightning'],
            ['solar:qr-code-bold-duotone', 'preview.onchain'],
            ['solar:user-rounded-bold-duotone', 'preview.identity'],
            ['solar:server-bold-duotone', 'preview.node'],
          ].map(([icon, label]) => (
            <Stack
              key={label}
              direction="row"
              spacing={1}
              sx={(theme) => ({
                p: 1.25,
                borderRadius: 1,
                alignItems: 'center',
                border: `1px solid ${varAlpha(theme.vars.palette.common.whiteChannel, 0.08)}`,
                bgcolor: varAlpha(theme.vars.palette.common.whiteChannel, 0.04),
              })}
            >
              <Iconify icon={icon} width={20} sx={{ color: item.accent }} />
              <Typography variant="caption" noWrap>
                {t(label)}
              </Typography>
            </Stack>
          ))}
        </Box>
      </Stack>
    </Box>
  );
}
