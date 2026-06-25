'use client';

import type { SlideData } from '../welcome-carousel';

import { Box } from '@mui/material';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { markWelcomeComplete } from 'src/lib/swissknife';

import { WelcomeCarousel } from '../welcome-carousel';

const slides: SlideData[] = [
  {
    id: 'welcome',
    title: 'welcome.title',
    content: 'welcome.content',
    icon: 'solar:home-angle-bold-duotone',
    accent: 'primary.main',
    points: ['welcome.point_one', 'welcome.point_two', 'welcome.point_three'],
  },
  {
    id: 'custody',
    title: 'custody.title',
    content: 'custody.content',
    icon: 'solar:shield-keyhole-bold-duotone',
    accent: 'success.main',
    points: ['custody.point_one', 'custody.point_two', 'custody.point_three'],
  },
  {
    id: 'accessibility',
    title: 'accessibility.title',
    content: 'accessibility.content',
    icon: 'solar:server-bold-duotone',
    accent: 'info.main',
    points: ['accessibility.point_one', 'accessibility.point_two', 'accessibility.point_three'],
  },
  {
    id: 'multi-user',
    title: 'multi_user.title',
    content: 'multi_user.content',
    icon: 'solar:users-group-rounded-bold-duotone',
    accent: 'warning.main',
    points: ['multi_user.point_one', 'multi_user.point_two', 'multi_user.point_three'],
  },
];

export function WelcomeView() {
  const router = useRouter();

  const handleWelcomeComplete = async () => {
    try {
      await markWelcomeComplete();
      router.push(paths.auth.signUp);
    } catch (err) {
      handleActionError(err);
    }
  };

  return (
    <Box
      sx={{
        position: 'relative',
        width: '100vw',
        minHeight: '100dvh',
        overflow: { xs: 'auto', md: 'hidden' },
        display: 'flex',
        alignItems: 'center',
        bgcolor: 'background.default',
      }}
    >
      <WelcomeCarousel data={slides} onComplete={handleWelcomeComplete} />
    </Box>
  );
}
