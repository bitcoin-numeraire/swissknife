'use client';

import type { Engine } from '@tsparticles/engine';
import type { SlideData } from 'src/components/carousel';

import React, { useEffect } from 'react';
import { loadSlim } from '@tsparticles/slim';
import { useBoolean } from 'minimal-shared/hooks';
import { loadAbsorbersPlugin } from '@tsparticles/plugin-absorbers';
import Particles, { initParticlesEngine } from '@tsparticles/react';

import { Box } from '@mui/material';

import { WelcomeCarousel } from 'src/components/carousel';

import particleOptions from '../particles';

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

type WelcomeScreenProps = {
  onComplete: () => void;
};

export function WelcomeView({ onComplete }: WelcomeScreenProps) {
  const init = useBoolean(false);

  useEffect(() => {
    initParticlesEngine(async (engine: Engine) => {
      await loadSlim(engine);
      await loadAbsorbersPlugin(engine);
    }).then(() => {
      init.onTrue();
    });
  }, [init]);

  return (
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
      {init.value && <Box component={Particles} id="tsparticles" options={particleOptions} />}

      <WelcomeCarousel data={slides} onComplete={onComplete} />
    </Box>
  );
}
