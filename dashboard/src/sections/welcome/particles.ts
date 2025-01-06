import type { ISourceOptions } from '@tsparticles/engine';

const particleOptions: ISourceOptions = {
  key: 'bitcoin',
  name: 'Bitcoin',
  fullScreen: {
    enable: true,
  },
  particles: {
    number: {
      value: 200,
    },
    collisions: {
      enable: true,
    },
    color: {
      value: '#ffffff',
    },
    shape: {
      type: 'circle',
    },
    opacity: {
      value: {
        min: 0.1,
        max: 1,
      },
    },
    size: {
      value: {
        min: 1,
        max: 2,
      },
    },
    move: {
      enable: true,
      speed: 0.5,
      direction: 'top',
      straight: true,
      warp: true,
    },
  },
  interactivity: {
    events: {
      onClick: {
        enable: true,
        mode: 'push',
      },
    },
    modes: {
      push: {
        quantity: 10,
      },
    },
  },
  absorbers: {
    density: 300,
    opacity: 1.0,
    color: {
      value: '#FF9900',
    },
    draggable: true,
    size: {
      value: {
        min: 10,
        max: 20,
      },
      limit: 60,
    },
    position: {
      x: 50,
      y: 20,
    },
  },
};
export default particleOptions;
