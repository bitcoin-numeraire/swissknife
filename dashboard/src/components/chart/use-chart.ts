import type { Theme } from '@mui/material/styles';
import type { ChartOptions } from './types';

import { useMemo } from 'react';
import { merge } from 'es-toolkit';
import { varAlpha } from 'minimal-shared/utils';

import { useTheme } from '@mui/material/styles';

// ----------------------------------------------------------------------

export function useChart(updatedOptions?: ChartOptions): ChartOptions {
  const theme = useTheme();

  const baseOptions = useMemo(() => baseChartOptions(theme), [theme]);

  return useMemo(
    () => (updatedOptions ? merge(baseOptions, updatedOptions) : baseOptions),
    [baseOptions, updatedOptions]
  );
}

// ----------------------------------------------------------------------

const baseChartOptions = (theme: Theme): ChartOptions => {
  const labelStyles = {
    total: {
      show: true,
      label: 'Total',
      color: theme.vars.palette.text.secondary,
      fontSize: theme.typography.subtitle2.fontSize as string,
      fontWeight: theme.typography.subtitle2.fontWeight,
    },
    value: {
      offsetY: 8,
      color: theme.vars.palette.text.primary,
      fontSize: theme.typography.h4.fontSize as string,
      fontWeight: theme.typography.h4.fontWeight,
    },
  };

  return {
    /** **************************************
     * Chart
     * https://apexcharts.com/docs/options/chart/animations/
     *************************************** */
    chart: {
      toolbar: { show: false },
      zoom: { enabled: false },
      parentHeightOffset: 0,
      fontFamily: theme.typography.fontFamily,
      foreColor: theme.vars.palette.text.disabled,
      animations: {
        enabled: true,
        speed: 360,
        animateGradually: { enabled: true, delay: 120 },
        dynamicAnimation: { enabled: true, speed: 360 },
      },
    },

    /** **************************************
     * Colors
     * https://apexcharts.com/docs/options/colors/
     *************************************** */
    colors: [
      theme.palette.primary.main,
      theme.palette.warning.main,
      theme.palette.info.main,
      theme.palette.error.main,
      theme.palette.success.main,
      theme.palette.warning.dark,
      theme.palette.success.darker,
      theme.palette.info.dark,
      theme.palette.info.darker,
    ],

    /** **************************************
     * States
     * https://apexcharts.com/docs/options/states/
     *************************************** */
    states: {
      hover: {
        filter: { type: 'darken' },
      },
      active: {
        filter: { type: 'darken' },
      },
    },

    /** **************************************
     * Fill
     * https://apexcharts.com/docs/options/fill/
     *************************************** */
    fill: {
      opacity: 1,
      gradient: {
        type: 'vertical',
        shadeIntensity: 0,
        opacityFrom: 0.4,
        opacityTo: 0,
        stops: [0, 100],
      },
    },

    /** **************************************
     * Data labels
     * https://apexcharts.com/docs/options/datalabels/
     *************************************** */
    dataLabels: {
      enabled: false,
    },

    /** **************************************
     * Stroke
     * https://apexcharts.com/docs/options/stroke/
     *************************************** */
    stroke: {
      width: 2.5,
      curve: 'smooth',
      lineCap: 'round',
    },

    /** **************************************
     * Grid
     * https://apexcharts.com/docs/options/grid/
     *************************************** */
    grid: {
      strokeDashArray: 3,
      borderColor: theme.vars.palette.divider,
      padding: { top: 0, right: 0, bottom: 0 },
      xaxis: { lines: { show: false } },
    },

    /** **************************************
     * Axis
     * https://apexcharts.com/docs/options/xaxis/
     * https://apexcharts.com/docs/options/yaxis/
     *************************************** */
    xaxis: {
      axisBorder: { show: false },
      axisTicks: { show: false },
    },
    yaxis: { tickAmount: 5 },

    /** **************************************
     * Markers
     * https://apexcharts.com/docs/options/markers/
     *************************************** */
    markers: {
      size: 0,
      strokeColors: theme.vars.palette.background.paper,
    },

    /** **************************************
     * Tooltip
     *************************************** */
    tooltip: {
      fillSeriesColor: false,
      x: { show: true },
    },

    /** **************************************
     * Legend
     * https://apexcharts.com/docs/options/legend/
     *************************************** */
    legend: {
      show: false,
      position: 'top',
      fontWeight: 500,
      fontSize: '13px',
      horizontalAlign: 'right',
      markers: { shape: 'circle' },
      labels: { colors: theme.vars.palette.text.primary },
      itemMargin: { horizontal: 8, vertical: 8 },
    },

    /** **************************************
     * plotOptions
     *************************************** */
    plotOptions: {
      /**
       * bar
       * https://apexcharts.com/docs/options/plotoptions/bar/
       */
      bar: {
        borderRadius: 4,
        columnWidth: '48%',
        borderRadiusApplication: 'end',
      },
      /**
       * pie + donut
       * https://apexcharts.com/docs/options/plotoptions/pie/
       */
      pie: {
        customScale: 0.98,
        donut: {
          labels: {
            show: true,
            value: labelStyles.value,
            total: labelStyles.total,
          },
        },
      },
      /**
       * radialBar
       * https://apexcharts.com/docs/options/plotoptions/radialbar/
       */
      radialBar: {
        hollow: { margin: -8, size: '100%' },
        track: {
          margin: -8,
          strokeWidth: '50%',
          background: varAlpha(theme.vars.palette.grey['500Channel'], 0.16),
        },
        dataLabels: {
          value: labelStyles.value,
          total: labelStyles.total,
        },
      },
      /**
       * radar
       * https://apexcharts.com/docs/options/plotoptions/radar/
       */
      radar: {
        polygons: {
          fill: { colors: ['transparent'] },
          strokeColors: theme.vars.palette.divider,
          connectorColors: theme.vars.palette.divider,
        },
      },
      /**
       * polarArea
       * https://apexcharts.com/docs/options/plotoptions/polararea/
       */
      polarArea: {
        rings: {
          strokeColor: theme.vars.palette.divider,
        },
        spokes: {
          connectorColors: theme.vars.palette.divider,
        },
      },
      /**
       * heatmap
       * https://apexcharts.com/docs/options/plotoptions/heatmap/
       */
      heatmap: {
        distributed: true,
      },
    },

    /** **************************************
     * Responsive
     * https://apexcharts.com/docs/options/responsive/
     *************************************** */
    responsive: [
      {
        // sm ~ 600
        breakpoint: theme.breakpoints.values.sm,
        options: {
          plotOptions: {
            bar: { borderRadius: 3, columnWidth: '80%' },
          },
        },
      },
      {
        // md ~ 900
        breakpoint: theme.breakpoints.values.md,
        options: {
          plotOptions: {
            bar: { columnWidth: '60%' },
          },
        },
      },
    ],
  };
};
