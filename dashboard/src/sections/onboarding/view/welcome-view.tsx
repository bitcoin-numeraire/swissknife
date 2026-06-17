'use client';

import type { Engine } from '@tsparticles/engine';
import type { SlideData } from '../welcome-carousel';

import { loadSlim } from '@tsparticles/slim';
import { loadAbsorbersPlugin } from '@tsparticles/plugin-absorbers';
import { Particles, ParticlesProvider, useParticlesProvider } from '@tsparticles/react';

import { Box } from '@mui/material';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { markWelcomeComplete } from 'src/lib/swissknife';

import particleOptions from '../particles';
import { WelcomeCarousel } from '../welcome-carousel';

const slides: SlideData[] = [
  { id: 'welcome', title: 'welcome.title', content: 'welcome.content' },
  {
    id: 'custody',
    title: 'custody.title',
    content: 'custody.content',
    icon: 'solar:shield-keyhole-minimalistic-outline',
  },
  {
    id: 'accessibility',
    title: 'accessibility.title',
    content: 'accessibility.content',
    icon: 'solar:cloud-bolt-outline',
  },
  {
    id: 'multi-user',
    title: 'multi_user.title',
    content: 'multi_user.content',
    icon: 'solar:users-group-two-rounded-outline',
  },
];

async function initParticles(engine: Engine) {
  await loadSlim(engine);
  await loadAbsorbersPlugin(engine);
}

function ParticlesBackground() {
  const { loaded } = useParticlesProvider();

  return loaded ? <Box component={Particles} id="tsparticles" options={particleOptions} /> : null;
}

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
    <ParticlesProvider init={initParticles}>
      <Box
        sx={{
          position: 'relative',
          width: '100vw',
          height: '100vh',
          overflow: 'hidden',
          display: 'flex',
          alignItems: 'center',
          textAlign: 'center',
        }}
      >
        <ParticlesBackground />

        <WelcomeCarousel data={slides} onComplete={handleWelcomeComplete} />
      </Box>
    </ParticlesProvider>
  );
}
